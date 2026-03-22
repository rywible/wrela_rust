#![forbid(unsafe_code)]

fn main() {
    let _entrypoint = wr_agentd::init_entrypoint();
    let _runtime = wr_agentd::target_runtime();
}
