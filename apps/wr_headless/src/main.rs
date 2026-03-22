#![forbid(unsafe_code)]

fn main() {
    let _ = wr_headless::init_entrypoint();
    let _ = wr_headless::target_runtime();
    std::process::exit(wr_headless::run(std::env::args().skip(1)));
}
