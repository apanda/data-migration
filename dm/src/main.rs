use iou::io_uring_queue_init;
use iou::*;
use libiouring as iou;
use std::net::TcpListener;
use std::os::unix::io::AsRawFd;
use std::ptr::null_mut;
fn main() -> std::io::Result<()> {
    const QDEPTH: u32 = 32;
    let connect = TcpListener::bind("127.0.0.1:8989")?;
    let cfd = connect.as_raw_fd();
    let mut ring: io_uring = Default::default();
    // Initialize io_uring, set things when necessary.
    unsafe {
        io_uring_queue_init(QDEPTH, &mut ring, 0);
        let entry = io_uring_get_sqe(&mut ring).unwrap();
        io_uring_prep_multishot_accept(entry, cfd, null_mut(), null_mut(), 0);
        set_sqe_data(entry, 22);
        let out = io_uring_submit(&mut ring);
        println!("Wait finished, got {}", out);
        let cqes = io_uring_wait_cqe_nr(&mut ring, 2).unwrap().unwrap();
        println!(
            "Wait finished, got {}, {}, {}",
            out,
            get_cqe_data(cqes.peek(0).unwrap()),
            get_cqe_data(cqes.peek(1).unwrap())
        );
        drop(cqes);
    };
    Ok(())
}
