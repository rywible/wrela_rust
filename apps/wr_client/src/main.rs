#![forbid(unsafe_code)]

fn main() {
    let _entrypoint = wr_client::init_entrypoint();
    let _runtime = wr_client::target_runtime();
}
