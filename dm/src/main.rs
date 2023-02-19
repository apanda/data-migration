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
        let mut cqe: *mut io_uring_cqe = null_mut();
        let out = __io_uring_get_cqe(&mut ring, &mut cqe, 0, 2, null_mut());
        let cqe2 = cqe.offset(1);
        println!(
            "Wait finished, got {}, {}, {}",
            out,
            get_cqe_data(&*cqe),
            get_cqe_data(&*cqe2)
        );
    };
    Ok(())
}
