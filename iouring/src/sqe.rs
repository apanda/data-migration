use std::os::fd::RawFd;

use super::*;

/// Return a SQE from `ring` or `None` if no empty SQEs are
/// available.
pub fn io_uring_get_sqe(ring: &mut IoUring) -> std::option::Option<&mut io_uring_sqe> {
    let sq = &mut ring.ring.sq;
    let next = sq.sqe_tail + 1;
    let shift = if (ring.ring.flags & IORING_SETUP_SQE128) != 0 {
        1
    } else {
        0
    };

    // Sigh nightly only
    unsafe {
        let skhead = AtomicU32::from_mut(&mut *sq.khead);
        let head = if (ring.ring.flags & IORING_SETUP_SQPOLL) == 0 {
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
}

fn io_uring_prep_rw(
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
    let t = unsafe { sqe.__bindgen_anon_6.__bindgen_anon_1.as_mut() };
    t.addr3 = 0;
    t.__pad2[0] = 0;
}

/// Prepare an accept requrest in the given SQE.
///
/// # Safety
/// We do not validate the `addr` and `len` fields,
/// but they must be `null` or point to valid memory.
pub fn io_uring_prep_accept(
    sqe: &mut io_uring_sqe,
    fd: i32,
    addr: *mut libc::sockaddr,
    len: *mut libc::socklen_t,
    flags: u32,
) {
    io_uring_prep_rw(IORING_OP_ACCEPT, sqe, fd, addr as usize, 0, len as u64);
    sqe.__bindgen_anon_3.accept_flags = flags;
}

pub fn io_uring_prep_multishot_accept(
    sqe: &mut io_uring_sqe,
    fd: i32,
    addr: *mut libc::sockaddr,
    len: *mut libc::socklen_t,
    flags: u32,
) {
    io_uring_prep_accept(sqe, fd, addr, len, flags);
    sqe.ioprio |= IORING_ACCEPT_MULTISHOT as u16;
}

/// Prepare a splice command. Either `fd_in` or `fd_out` must be a pipe.
/// If `fd_in` is a pipe, `off_in` must be set to -1.
///
/// If `fd_in` does not refer to a pipe, and `off_in` is -1, then `nbytes` are
/// read from `fd_in` starting from the current file offset which is incremented
/// appropriated.
///
/// If `fd_in` does not refer to a pipe, and `off_in` is not -1, then the read starts
/// at offset `off_in`.
///
/// This operation might fail with an EINVAl.
pub fn io_uring_prep_splice(
    sqe: &mut io_uring_sqe,
    fd_in: RawFd,
    off_in: i64,
    fd_out: RawFd,
    off_out: i64,
    nbytes: u32,
    flags: u32,
) {
    io_uring_prep_rw(IORING_OP_SPLICE, sqe, fd_out, 0, nbytes, off_out as u64);
    sqe.__bindgen_anon_3.splice_flags = flags;
    sqe.__bindgen_anon_5.splice_fd_in = fd_in;
    sqe.__bindgen_anon_2.splice_off_in = off_in as u64;
}

pub fn io_uring_prep_tee(
    sqe: &mut io_uring_sqe,
    fd_in: RawFd,
    fd_out: RawFd,
    nbytes: u32,
    flags: u32,
) {
    io_uring_prep_rw(IORING_OP_TEE, sqe, fd_out, 0, nbytes, 0);
    sqe.__bindgen_anon_2.splice_off_in = 0;
    sqe.__bindgen_anon_5.splice_fd_in = fd_in;
    sqe.__bindgen_anon_3.splice_flags = flags;
}

/// Set SQE data, this shows up in the corresponding CQE allowing
/// returns to be correlated with requests.
pub fn set_sqe_data(sqe: &mut io_uring_sqe, data: u64) {
    sqe.user_data = data;
}

/// Returns user data from `sqe`.
pub fn get_sqe_data(sqe: &io_uring_sqe) -> u64 {
    sqe.user_data
}
