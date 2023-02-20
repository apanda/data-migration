#![feature(atomic_from_mut)]
#![feature(let_chains)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::marker::PhantomPinned;
use std::ptr::{null_mut, NonNull};
use std::sync::atomic::{AtomicU32, Ordering};

include! {concat!(env!("OUT_DIR"), "/iouring-sys.rs")}

mod cqe;
pub use cqe::*;
mod sqe;
pub use sqe::*;

/// An IoUring structure, mostly so we can tell the
/// Rust type system a bit more about our constraints.
pub struct IoUring {
    pub(crate) ring: io_uring,
    _pin: PhantomPinned,
}

impl IoUring {
    pub fn init(depth: isize) -> IoUring {
        let mut r = IoUring {
            ring: Default::default(),
            _pin: Default::default(),
        };
        unsafe { io_uring_queue_init(depth as u32, &mut r.ring, 0) };
        r
    }

    /// Returns the underlying `io_uring` so one can directly
    /// call liburing methods. This is unsafe for obvious reasons,
    /// and is a way to get around my laziness.
    pub unsafe fn get_ring_ptr(&mut self) -> *mut io_uring {
        &mut self.ring
    }

    /// Submit pending SQEs.
    ///
    /// Returns number of submitted tasks.
    pub fn submit(&mut self) -> i32 {
        unsafe { io_uring_submit(&mut self.ring) }
    }

    /// Returns the number of SQEs that are ready but not
    /// consumed by the kernel. Note, we do not mutate `ring`
    /// but need to accept it this way to impose a barrier.
    #[inline(always)]
    pub fn io_uring_sq_ready(&mut self) -> u32 {
        unsafe {
            let kh = if self.ring.flags & IORING_SETUP_SQPOLL != 0 {
                AtomicU32::from_mut(&mut *self.ring.sq.khead).load(Ordering::Acquire)
            } else {
                *self.ring.sq.khead
            };
            let tail = self.ring.sq.sqe_tail;
            tail - kh
        }
    }

    /// Returns the number of SQEs avaialble.
    #[inline(always)]
    pub fn io_uring_sq_available(&mut self) -> u32 {
        self.ring.sq.ring_entries - self.io_uring_sq_ready()
    }

    /// Return number of unconsumed CQEs.
    #[inline(always)]
    pub fn io_uring_cq_ready(&mut self) -> u32 {
        let kt = unsafe { AtomicU32::from_mut(&mut *self.ring.cq.ktail).load(Ordering::Acquire) };
        let kh: u32 = unsafe { *self.ring.cq.khead };
        kt - kh
    }

    /// Return if there are overlow entries that need to be flushed
    /// to CQ (indicating the application is not keeping up).
    pub fn io_uring_cq_has_overflow(&mut self) -> bool {
        let flag =
            unsafe { AtomicU32::from_mut(&mut *self.ring.sq.kflags).load(Ordering::Relaxed) };
        (flag & IORING_SQ_CQ_OVERFLOW) != 0
    }
}
