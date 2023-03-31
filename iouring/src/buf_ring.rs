extern crate static_assertions as sa;
use std::alloc::{alloc, dealloc, Layout};
use std::mem::size_of;
use std::sync::atomic::AtomicU16;

use super::*;

pub struct BufRing {
    buffers: *mut io_uring_buf_ring,
    layout: Layout,
    reg: io_uring_buf_reg,
    io_bufs: *mut u8,
    io_layout: Layout,
    entry_size: usize,
    pub mask: u32,
}

impl Drop for BufRing {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.buffers as *mut u8, self.layout);
            dealloc(self.io_bufs, self.io_layout);
        }
    }
}

impl BufRing {
    #[inline(always)]
    pub fn ring_mask(&self) -> u32 {
        self.reg.ring_entries - 1
    }

    #[inline(always)]
    pub fn ring_init(&mut self) {
        unsafe {
            (*self.buffers)
                .__bindgen_anon_1
                .__bindgen_anon_1
                .as_mut()
                .tail = 0
        };
    }

    #[inline(always)]
    pub fn ring_update_tail(&mut self, tail: u16) {
        unsafe {
            let atomic_tail = AtomicU16::from_mut(
                &mut (*self.buffers)
                    .__bindgen_anon_1
                    .__bindgen_anon_1
                    .as_mut()
                    .tail,
            );
            atomic_tail.store(tail, Ordering::Release);
        };
    }

    #[inline(always)]
    unsafe fn get_buffers(&mut self) -> *mut io_uring_buf {
        // [apanda] This is too clever, so an explanation follows.
        // This (and the very ugly) bindgen definition are because
        // the kernel is cleverly trying to space makes `io_uring_buf`
        // and `io_uring_buf_ring` the same size, and uses the same
        // shape for both. The C struct looks like
        // ```
        // struct io_uring_buf_ring {
        //	union {
        // 		struct {
        // 			__u64	resv1;
        // 			__u32	resv2;
        // 			__u16	resv3;
        // 			__u16	tail;
        // 		};
        // 		struct io_uring_buf	bufs[0];
        // 	};
        // };
        // ```
        // Indeed, this is why  allocating sizeof(io_uring_buf_ring) * entries (below) suffices
        // for allocating a ring of entires size. Now, we could probably create a cleaner Rust
        // struct here, but I am going to use bindgen for now.

        sa::const_assert!(size_of::<io_uring_buf>() == size_of::<io_uring_buf_ring>());
        self.buffers as *mut io_uring_buf
    }

    #[inline(always)]
    pub unsafe fn get_tail(&self) -> usize {
        (*self.buffers)
            .__bindgen_anon_1
            .__bindgen_anon_1
            .as_ref()
            .tail as usize
    }

    #[inline(always)]
    unsafe fn set_buffer_at_idx(&mut self, offset: usize, addr: *mut u8, len: usize, bid: u16) {
        // struct io_uring_buf {
        //     __u64	addr;
        //     __u32	len;
        //     __u16	bid;
        //     __u16	resv;
        // };
        let offset = (self.get_tail() + offset) & (self.ring_mask() as usize);
        let buffer = self.get_buffers().offset(offset as isize);
        (*buffer).addr = addr as u64;
        (*buffer).len = len as u32;
        (*buffer).bid = bid;
    }

    #[inline(always)]
    unsafe fn add_all_buffers(&mut self) {
        for i in 0..self.reg.ring_entries {
            self.set_buffer_at_idx(
                i as usize,
                self.io_bufs.offset(self.entry_size as isize * i as isize),
                self.entry_size,
                i as u16,
            );
        }
        let new_tail = self.get_tail() + self.reg.ring_entries as usize;
        self.ring_update_tail(new_tail as u16);
    }

    /// Initialize a buffer ring with a given group ID and entries.
    /// Note, for convenience this also allocates
    pub fn init_with_group_id(
        ring: &mut IoUring,
        group_id: u16,
        entries: u32,
        entry_size: usize,
    ) -> std::io::Result<Self> {
        if entries & (entries - 1) != 0 {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "entries must be a power of 2",
            ))
        } else {
            let layout =
                Layout::from_size_align(size_of::<io_uring_buf_ring>() * (entries as usize), 4096)
                    .unwrap();
            let buffers = unsafe { alloc(layout) as *mut io_uring_buf_ring };
            let io_layout = Layout::from_size_align((entries as usize) * entry_size, 4096).unwrap();
            let io_bufs = unsafe { alloc(io_layout) as *mut u8 };
            let mut ret = BufRing {
                buffers: buffers,
                layout: layout,
                reg: io_uring_buf_reg {
                    ring_addr: buffers as u64,
                    ring_entries: entries,
                    bgid: group_id,
                    ..Default::default()
                },
                io_bufs: io_bufs,
                io_layout: io_layout,
                entry_size: entry_size,
                mask: entries - 1,
            };
            unsafe {
                let out = io_uring_register_buf_ring(ring.get_ring_ptr(), &mut ret.reg, 0);
                if out != 0 {
                    panic!("Registration failed");
                }
                ret.ring_init();
                ret.add_all_buffers();
            }
            Ok(ret)
        }
    }
}
