fn main() {
    use std::env;
    use std::path::PathBuf;
    println!("cargo:rustc-link-lib=uring");
    const INCLUDE: &str = r#"
#include <liburing.h>
#include <liburing/compat.h>
#include <liburing/io_uring.h>
#include <liburing/barrier.h>
    "#;

    let outdir = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindgen::Builder::default()
        .header_contents("include-file.h", INCLUDE)
        .ctypes_prefix("libc")
        .prepend_enum_name(false)
        .derive_default(true)
        .generate_comments(true)
        // .allowlist_type("io_uring_.*|io_.qring_.*|__kernel_timespec|open_how")
        // .allowlist_var("__NR_io_uring.*|IOSQE_.*|IORING_.*|IO_URING_.*|SPLICE_F_FD_IN_FIXED")
        //.use_core()
        .generate()
        .unwrap()
        .write_to_file(outdir.join("iouring-sys.rs"))
        .unwrap();
}
