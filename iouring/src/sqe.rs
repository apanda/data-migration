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

    /// Get raw SQE pointer.
    ///
    /// # Safety
    /// The IoUring must remain valid if the SQE is used.
    /// Otherwise an illegal memory access is likely.
    pub unsafe fn get_sqe(&self) -> *mut io_uring_sqe {
        self.sqe
    }

    /// Set SQE data, this shows up in the corresponding CQE allowing
    /// returns to be correlated with requests.
    pub fn set_sqe_data(self, data: u64) -> Self {
        let sqe = unsafe { &mut (*self.sqe) };
        sqe.user_data = data;
        self
    }

    /// Returns user data from `sqe`.
    pub fn get_sqe_data(&self) -> u64 {
        let sqe = unsafe { &(*self.sqe) };
        sqe.user_data
    }

    /// Link this SQE to the next SQE. This ensure that the
    /// next SQE will not start until the current one completes
    /// or errors out.
    /// 
    /// One can use this to form a chain of SQEs.
    pub fn set_link(self) -> Self {
        unsafe { (*(self.sqe)).flags |= 1u8 << IOSQE_IO_LINK_BIT };
        self
    }

    /// Link this SQE to the next SQE, but do not execute
    /// if the SQE errors out.
    pub fn set_hard_link(self) -> Self {
        unsafe { (*(self.sqe)).flags |= 1u8 << IOSQE_IO_HARDLINK_BIT };
        self
    }

    /// Indicate that we know that this call will always require waiting,
    /// and the kernel should always treat this as an async call.
    pub fn set_async(self) -> Self {
        unsafe { (*(self.sqe)).flags |= 1u8 << IOSQE_ASYNC_BIT };
        self
    }

    /// Have this SQE act as a barrier: it will not execute until all previous
    /// submissions have completed, and no later SQEs will execute until it
    /// executes.
    pub fn set_drain(self) -> Self {
        unsafe { (*(self.sqe)).flags |= 1u8 << IOSQE_IO_DRAIN_BIT };
        self
    }

    /// Do not generate a CQE if this request succeeds.
    pub fn set_no_cqe(self) -> Self {
        unsafe { (*(self.sqe)).flags |= 1u8 << IOSQE_CQE_SKIP_SUCCESS_BIT };
        self
    }

    /// Use a buffer group for this SQE if available.
    pub fn set_buffer_select(self) -> Self {
        unsafe { (*(self.sqe)).flags |= 1u8 << IOSQE_BUFFER_SELECT_BIT };
        self
    }
    
    /// The FD is a previously registered file or direct FD, rather than a
    /// normal file descriptor.
    pub fn set_fixed_file(self) -> Self {
        unsafe { (*(self.sqe)).flags |= 1u8 << IOSQE_FIXED_FILE_BIT };
        self
    }

    /// Initialize a SQE. This is only available
    /// to work around all the missing elements.
    ///
    /// # Safety
    /// We don't check where the `sqe` comes from,
    /// and might thus be unsafe.
    pub unsafe fn io_uring_prep_rw(
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
        let t = sqe.__bindgen_anon_6.__bindgen_anon_1.as_mut();
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
    ) -> Self {
        let sqe = unsafe { &mut (*self.sqe) };
        unsafe { Self::io_uring_prep_rw(sqe, IORING_OP_ACCEPT, fd, addr as usize, 0, len as u64) };
        sqe.__bindgen_anon_3.accept_flags = flags;
        self
    }

    pub fn io_uring_prep_multishot_accept(
        self,
        fd: i32,
        addr: *mut libc::sockaddr,
        len: *mut libc::socklen_t,
        flags: u32,
    ) -> Self {
        let s = self.io_uring_prep_accept(fd, addr, len, flags);
        let sqe = unsafe { &mut (*s.sqe) };
        sqe.ioprio |= IORING_ACCEPT_MULTISHOT as u16;
        s
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
    ) -> Self {
        let sqe = unsafe { &mut (*self.sqe) };
        unsafe { Self::io_uring_prep_rw(sqe, IORING_OP_SPLICE, fd_out, 0, nbytes, off_out as u64) };
        sqe.__bindgen_anon_3.splice_flags = flags;
        sqe.__bindgen_anon_5.splice_fd_in = fd_in;
        sqe.__bindgen_anon_2.splice_off_in = off_in as u64;
        self 
    }

    pub fn io_uring_prep_tee(self, fd_in: RawFd, fd_out: RawFd, nbytes: u32, flags: u32) -> Self {
        let sqe = unsafe { &mut (*self.sqe) };
        unsafe { Self::io_uring_prep_rw(sqe, IORING_OP_TEE, fd_out, 0, nbytes, 0) };
        sqe.__bindgen_anon_2.splice_off_in = 0;
        sqe.__bindgen_anon_5.splice_fd_in = fd_in;
        sqe.__bindgen_anon_3.splice_flags = flags;
        self
    }

    /// Indicate that we are done with the SQE.
    pub fn finalize(self) {
        drop(self);
    }

}
