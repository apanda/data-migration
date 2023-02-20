#![feature(atomic_from_mut)]
#![feature(let_chains)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, Ordering};

include! {concat!(env!("OUT_DIR"), "/iouring-sys.rs")}

// This is an internal function that I am copying rather
// than using liburing's ffi form
pub unsafe fn io_uring_get_sqe(ring: &mut io_uring) -> std::option::Option<&mut io_uring_sqe> {
    let sq = &mut ring.sq;
    let next = sq.sqe_tail + 1;
    let shift = if (ring.flags & IORING_SETUP_SQE128) != 0 {
        1
    } else {
        0
    };
    // Sigh nightly only
    let skhead = AtomicU32::from_mut(&mut *sq.khead);
    let head = if (ring.flags & IORING_SETUP_SQPOLL) == 0 {
        skhead.load(Ordering::Relaxed)
    } else {
        skhead.load(Ordering::Acquire)
    };
    if next - head <= sq.ring_entries {
        let current = sq.sqe_tail;
        sq.sqe_tail = next;
        Some(&mut *(sq.sqes.offset(((current & sq.ring_mask) << shift) as isize)))
    } else {
        None
    }
}

#[inline(always)]
pub unsafe fn io_uring_cq_advance(ring: &mut io_uring, nr: u32) {
    if nr > 0 {
        let ckhead = AtomicU32::from_mut(&mut *ring.cq.khead);
        ckhead.fetch_add(nr, Ordering::AcqRel);
    }
}

#[inline(always)]
unsafe fn io_uring_cqe_seen(ring: &mut io_uring) {
    io_uring_cq_advance(ring, 1)
}

/// A holder for a set of CQEs, potentially
/// returned by peek or get.
pub struct CqeHolder {
    cqes: *mut io_uring_cqe,
    // First valid CQE, we need this because
    // accessing CQEs after advancing is not
    // safe.
    begin: isize,
    // Bounds
    end: isize,
}

impl CqeHolder {
    /// Create a CQE holder.
    ///
    /// # Safety
    /// This function cannot check that the CQEs and length are
    /// correct, and instead assumes this is the case. This is
    /// of course unsafe, since it allows access to arbitrary
    /// memory.
    pub unsafe fn init(cqes: *mut io_uring_cqe, available: isize) -> CqeHolder {
        CqeHolder {
            cqes: cqes,
            begin: 0,
            end: available,
        }
    }

    pub fn peek(&self, idx: isize) -> Option<&io_uring_cqe> {
        if idx >= self.begin && idx < self.end {
            unsafe { Some(&*self.cqes.offset(idx)) }
        } else {
            None
        }
    }

    pub fn peek_mut(&mut self, idx: isize) -> Option<&mut io_uring_cqe> {
        if idx >= self.begin && idx < self.end {
            unsafe { Some(&mut *self.cqes.offset(idx)) }
        } else {
            None
        }
    }

    pub fn available(&self) -> isize {
        self.end - self.begin
    }
}

pub unsafe fn io_uring_peek_cqe(ring: &mut io_uring) -> Result<Option<CqeHolder>, std::io::Error> {
    let shift = if ring.flags & IORING_SETUP_CQE32 != 0 {
        0
    } else {
        1
    };
    let cqtail = AtomicU32::from_mut(&mut *ring.cq.ktail);
    let cqhead = AtomicU32::from_mut(&mut *ring.cq.khead);
    const LIBURING_UDATA_TIMEOUT: u64 = u64::MAX;
    let mut err = 0;
    loop {
        let tail = cqtail.load(Ordering::Acquire);
        let head = cqhead.load(Ordering::Relaxed);
        let available = tail - head;
        if available > 0 {
            let cqes = ring
                .cq
                .cqes
                .offset(((head & ring.cq.ring_mask) >> shift) as isize);
            // Timeout handling, consume cqes that indicate timeouts
            if ring.features & IORING_FEAT_EXT_ARG == 0
                && (*cqes).user_data == LIBURING_UDATA_TIMEOUT
            {
                if (*cqes).res < 0 {
                    err = (*cqes).res
                };
                io_uring_cq_advance(ring, 1);
                if err == 0 {
                    continue;
                } else {
                    return Err(std::io::Error::from_raw_os_error(-err));
                }
            } else {
                return Ok(Some(CqeHolder::init(cqes, available as isize)));
            }
        } else {
            return Ok(None);
        }
    }
}

pub unsafe fn io_uring_wait_cqe_nr(
    ring: &mut io_uring,
    nr: u32,
) -> Result<Option<CqeHolder>, std::io::Error> {
    let peek = io_uring_peek_cqe(ring);
    if let Ok(Some(c)) = &peek &&
        c.available() >= (nr as isize) {
            peek
    } else if let Err(_) = &peek {
        peek
    } else {
        let mut cqe_ptr:*mut io_uring_cqe = null_mut();
        let ret = __io_uring_get_cqe(ring, &mut cqe_ptr, 0, nr, null_mut());
        if ret == 0 {
            Ok(Some(CqeHolder::init(cqe_ptr, nr as isize)))
        } else if ret < 0 {
            Err(std::io::Error::from_raw_os_error(-ret))
        } else {
            panic!("Unexpected return value.")
        }
    }
}

pub unsafe fn io_uring_wait_cqe(ring: &mut io_uring) -> Result<Option<CqeHolder>, std::io::Error> {
    io_uring_wait_cqe_nr(ring, 1)
}

unsafe fn io_uring_prep_rw(
    op: io_uring_op,
    sqe: &mut io_uring_sqe,
    fd: i32,
    addr: usize,
    len: u32,
    offset: u64,
) {
    sqe.opcode = (op & 0xff) as u8;
    sqe.flags = 0;
    sqe.ioprio = 0;
    sqe.fd = fd;
    sqe.__bindgen_anon_1.off = offset;
    sqe.__bindgen_anon_2.addr = addr as u64;
    sqe.len = len;
    sqe.__bindgen_anon_3.rw_flags = 0;
    sqe.__bindgen_anon_4.buf_index = 0;
    sqe.personality = 0;
    sqe.__bindgen_anon_5.file_index = 0;
    let t = sqe.__bindgen_anon_6.__bindgen_anon_1.as_mut();
    t.addr3 = 0;
    t.__pad2[0] = 0;
}

pub unsafe fn io_uring_prep_accept(
    sqe: &mut io_uring_sqe,
    fd: i32,
    addr: *mut libc::sockaddr,
    len: *mut libc::socklen_t,
    flags: u32,
) {
    io_uring_prep_rw(IORING_OP_ACCEPT, sqe, fd, addr as usize, 0, len as u64);
    sqe.__bindgen_anon_3.accept_flags = flags;
}

pub unsafe fn io_uring_prep_multishot_accept(
    sqe: &mut io_uring_sqe,
    fd: i32,
    addr: *mut libc::sockaddr,
    len: *mut libc::socklen_t,
    flags: u32,
) {
    io_uring_prep_accept(sqe, fd, addr, len, flags);
    sqe.ioprio |= IORING_ACCEPT_MULTISHOT as u16;
}

/// Set SQE data, this shows up in the corresponding CQE allowing
/// returns to be correlated with requests.
pub unsafe fn set_sqe_data(sqe: &mut io_uring_sqe, data: u64) {
    sqe.user_data = data;
}

/// Returns SQE data
pub unsafe fn get_sqe_data(sqe: &io_uring_sqe) -> u64 {
    sqe.user_data
}

/// Returns CQE data
pub unsafe fn get_cqe_data(cqe: &io_uring_cqe) -> u64 {
    cqe.user_data
}
