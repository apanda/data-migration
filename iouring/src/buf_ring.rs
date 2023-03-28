use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;
use std::mem::size_of;

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
            }
            Ok(ret)
        }
    }

    pub fn ring_mask(&self) -> u32 {
        self.reg.ring_entries - 1
    }

    pub fn ring_init(&mut self) {
        unsafe {
            (*self.buffers)
                .__bindgen_anon_1
                .__bindgen_anon_1
                .as_mut()
                .tail = 0
        };
    }
}
