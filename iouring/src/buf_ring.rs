use std::marker::PhantomData;
use std::alloc::{alloc, dealloc, Layout};
use std::mem::size_of;

use super::*;

pub struct BufRing {
    buffers: *mut io_uring_buf_ring,
    layout: Layout,
    reg: io_uring_buf_reg,
}

impl Drop for BufRing {
    fn drop(&mut self) {
       unsafe {
        dealloc(self.buffers as *mut u8, self.layout)
       }; 
    }
}

impl BufRing {
    pub fn init_with_group_id(ring: &mut IoUring, group_id: u16, entries: u32) -> Self {
        
        let layout = Layout::from_size_align(
                            size_of::<io_uring_buf_ring>() * (entries as usize),
                            4096).unwrap();
        let buffers = unsafe {alloc(layout) as *mut io_uring_buf_ring};
        let mut ret = BufRing {
            buffers: buffers,
            layout: layout,
            reg: io_uring_buf_reg {
                ring_addr: buffers as u64,
                ring_entries: entries,
                bgid: group_id,
                ..Default::default()
            } 
        };
        unsafe {
            let out = io_uring_register_buf_ring(ring.get_ring_ptr(), &mut ret.reg, 0);
            if out != 0 {
                panic!("Registration failed");
            }
            
        }
        ret
    }
}