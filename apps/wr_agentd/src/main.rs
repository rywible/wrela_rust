#![forbid(unsafe_code)]

fn main() {
    std::process::exit(wr_agentd::run(std::env::args().skip(1)));
}
