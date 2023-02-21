use std::{marker::PhantomData, os::fd::RawFd};

use super::*;

/// Rust friendly representation of a SQE
pub struct Sqe<'a> {
    sqe: *mut io_uring_sqe,
    _phantom: PhantomData<&'a ()>,
}

impl Sqe<'_> {
    pub(crate) unsafe fn init<'a>(sqe: *mut io_uring_sqe) -> Sqe<'a> {
        Sqe {
            sqe,
            _phantom: Default::default(),
        }
    }
    fn io_uring_prep_rw(
        sqe: &mut io_uring_sqe,
        op: io_uring_op,
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
        self,
        fd: i32,
        addr: *mut libc::sockaddr,
        len: *mut libc::socklen_t,
        flags: u32,
    ) {
        let sqe = unsafe { &mut (*self.sqe) };
        Self::io_uring_prep_rw(sqe, IORING_OP_ACCEPT, fd, addr as usize, 0, len as u64);
        sqe.__bindgen_anon_3.accept_flags = flags;
    }

    pub fn io_uring_prep_multishot_accept(
        self,
        fd: i32,
        addr: *mut libc::sockaddr,
        len: *mut libc::socklen_t,
        flags: u32,
    ) {
        let sqe = unsafe { &mut (*self.sqe) };
        Self::io_uring_prep_rw(sqe, IORING_OP_ACCEPT, fd, addr as usize, 0, len as u64);
        sqe.__bindgen_anon_3.accept_flags = flags;
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
        self,
        fd_in: RawFd,
        off_in: i64,
        fd_out: RawFd,
        off_out: i64,
        nbytes: u32,
        flags: u32,
    ) {
        let sqe = unsafe { &mut (*self.sqe) };
        Self::io_uring_prep_rw(sqe, IORING_OP_SPLICE, fd_out, 0, nbytes, off_out as u64);
        sqe.__bindgen_anon_3.splice_flags = flags;
        sqe.__bindgen_anon_5.splice_fd_in = fd_in;
        sqe.__bindgen_anon_2.splice_off_in = off_in as u64;
    }

    pub fn io_uring_prep_tee(self, fd_in: RawFd, fd_out: RawFd, nbytes: u32, flags: u32) {
        let sqe = unsafe { &mut (*self.sqe) };
        Self::io_uring_prep_rw(sqe, IORING_OP_TEE, fd_out, 0, nbytes, 0);
        sqe.__bindgen_anon_2.splice_off_in = 0;
        sqe.__bindgen_anon_5.splice_fd_in = fd_in;
        sqe.__bindgen_anon_3.splice_flags = flags;
    }

    /// Set SQE data, this shows up in the corresponding CQE allowing
    /// returns to be correlated with requests.
    pub fn set_sqe_data(&mut self, data: u64) {
        let sqe = unsafe { &mut (*self.sqe) };
        sqe.user_data = data;
    }

    /// Returns user data from `sqe`.
    pub fn get_sqe_data(&self) -> u64 {
        let sqe = unsafe { &(*self.sqe) };
        sqe.user_data
    }
}
