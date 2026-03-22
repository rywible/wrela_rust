#![forbid(unsafe_code)]

use wr_core::{CrateBoundary, CrateEntryPoint};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_render_api", CrateBoundary::Subsystem, false)
}
