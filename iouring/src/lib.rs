#![feature(atomic_from_mut)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include! {concat!(env!("OUT_DIR"), "/iouring-sys.rs")}

// TODO: Revisit this effort once we have basics running.

// This is an internal function that I am copying rather
// than using liburing's ffi form
pub unsafe fn io_uring_get_sqe(ring: &mut io_uring) -> std::option::Option<*mut io_uring_sqe> {
    let sq = &mut ring.sq;
    let next = sq.sqe_tail + 1;
    let shift = if (ring.flags & IORING_SETUP_SQE128) != 0 {
        1
    } else {
        0
    };
    // Sigh nightly only
    let skhead = std::sync::atomic::AtomicU32::from_mut(&mut *sq.khead);
    let head = if (ring.flags & IORING_SETUP_SQPOLL) == 0 {
        skhead.load(std::sync::atomic::Ordering::Relaxed)
    } else {
        skhead.load(std::sync::atomic::Ordering::Acquire)
    };
    if next - head <= sq.ring_entries {
        let current = sq.sqe_tail;
        sq.sqe_tail = next;
        Some(sq.sqes.offset(((current & sq.ring_mask) << shift) as isize))
    } else {
        None
    }
}

unsafe fn io_uring_prep_rw(
    op: io_uring_op,
    sqe: *mut io_uring_sqe,
    fd: i32,
    addr: usize,
    len: u32,
    offset: u64,
) {
    (*sqe).opcode = (op & 0xff) as u8;
    (*sqe).flags = 0;
    (*sqe).ioprio = 0;
    (*sqe).fd = fd;
    (*sqe).__bindgen_anon_1.off = offset;
    (*sqe).__bindgen_anon_2.addr = addr as u64;
    (*sqe).len = len;
    (*sqe).__bindgen_anon_3.rw_flags = 0;
    (*sqe).__bindgen_anon_4.buf_index = 0;
    (*sqe).personality = 0;
    (*sqe).__bindgen_anon_5.file_index = 0;
    let t = (*sqe).__bindgen_anon_6.__bindgen_anon_1.as_mut();
    t.addr3 = 0;
    t.__pad2[0] = 0;
}

pub unsafe fn io_uring_prep_accept(
    sqe: *mut io_uring_sqe,
    fd: i32,
    addr: *mut libc::sockaddr,
    len: *mut libc::socklen_t,
    flags: u32,
) {
    io_uring_prep_rw(IORING_OP_ACCEPT, sqe, fd, addr as usize, 0, len as u64);
    (*sqe).__bindgen_anon_3.accept_flags = flags;
}

pub unsafe fn io_uring_prep_multishot_accept(
    sqe: *mut io_uring_sqe,
    fd: i32,
    addr: *mut libc::sockaddr,
    len: *mut libc::socklen_t,
    flags: u32,
) {
    io_uring_prep_accept(sqe, fd, addr, len, flags);
    (*sqe).ioprio = (*sqe).ioprio | (IORING_ACCEPT_MULTISHOT as u16);
}
