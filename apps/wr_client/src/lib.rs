#![forbid(unsafe_code)]

use wr_core::{CrateBoundary, CrateEntryPoint};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_client", CrateBoundary::AppShell, true)
}

pub const fn target_runtime() -> CrateEntryPoint {
    wr_game::init_entrypoint()
}
