#![forbid(unsafe_code)]

fn main() {
    std::process::exit(xtask::run(std::env::args().skip(1)));
}
