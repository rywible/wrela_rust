#![forbid(unsafe_code)]

fn main() {
    let _entrypoint = wr_headless::init_entrypoint();
    let _runtime = wr_headless::target_runtime();
}
