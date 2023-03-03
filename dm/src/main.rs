use iou::*;
use libiouring as iou;
use std::mem::size_of;
use std::net::TcpListener;
use std::os::unix::io::AsRawFd;
use std::ptr::null_mut;
use std::alloc::{alloc, Layout};
fn main() -> std::io::Result<()> {
    const QDEPTH: u32 = 32;
    let connect = TcpListener::bind("127.0.0.1:8989")?;
    let cfd = connect.as_raw_fd();
    let mut ring = IoUring::init(QDEPTH as isize);
    const BUFS: usize = 64;
    let mut br = BufRing::init_with_group_id(&mut ring, 0xf, BUFS as u32);
    // Initialize io_uring, set things when necessary.
    let entry = ring.io_uring_get_sqe().unwrap();
    entry
        .io_uring_prep_multishot_accept(cfd, null_mut(), null_mut(), 0)
        .set_sqe_data(22)
        .finalize();
    let out = ring.submit();
    println!("Wait finished, got {}", out);
    let cqes = io_uring_wait_cqe_nr(&mut ring, 2).unwrap().unwrap();
    for i in 0..2 {
        let c = cqes.peek(i).unwrap();
        println!(
            "{} Wait finished, got {}, {}",
            i,
            c.get_cqe_data(),
            c.get_result()
        );
    };
    drop(cqes);
    println!(
        "After drop: CQEs {} SQEs R {} SQEs A {}",
        ring.io_uring_cq_ready(),
        ring.io_uring_sq_ready(),
        ring.io_uring_sq_available()
    );
    Ok(())
}
