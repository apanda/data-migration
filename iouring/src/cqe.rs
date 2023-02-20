
use super::*;
#[inline(always)]
unsafe fn io_uring_cq_advance(ring: &mut io_uring, nr: u32) {
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
    ring: NonNull<io_uring>,
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
    pub unsafe fn init(cqes: *mut io_uring_cqe, available: isize, ring: NonNull<io_uring>) -> CqeHolder {
        CqeHolder {
            ring: ring,
            cqes: cqes,
            begin: 0,
            end: available,
        }
    }

    #[inline(always)]
    pub fn peek(&self, idx: isize) -> Option<&io_uring_cqe> {
        if idx >= self.begin && idx < self.end {
            unsafe { Some(&*self.cqes.offset(idx)) }
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn peek_mut(&mut self, idx: isize) -> Option<&mut io_uring_cqe> {
        if idx >= self.begin && idx < self.end {
            unsafe { Some(&mut *self.cqes.offset(idx)) }
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn consume_one(&mut self) {
        unsafe {
            io_uring_cqe_seen(self.ring.as_mut())
        };
        self.begin += 1;
    }

    #[inline(always)]
    pub fn consume_all(&mut self) {
        unsafe {
            io_uring_cq_advance(self.ring.as_mut(), self.available() as u32)
        };
        self.begin = 0;
        self.end = 0;
    }

    pub fn available(&self) -> isize {
        self.end - self.begin
    }
}


impl Drop for CqeHolder {
    fn drop(&mut self) {
        self.consume_all()
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
                return Ok(Some(CqeHolder::init(cqes, available as isize, ring.into())));
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
    let p = io_uring_peek_cqe(ring);
    if let Ok(Some(c)) = &p &&
        c.available() >= nr as isize {
            p
    } else if let Err(_) = p {
        p
    } else {
        let mut cqe_ptr:*mut io_uring_cqe = null_mut();
        let ret = { 
            __io_uring_get_cqe(ring, &mut cqe_ptr, 0, nr, null_mut())
        };
        if ret == 0 {
            Ok(Some(CqeHolder::init(cqe_ptr, nr as isize, ring.into())))
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