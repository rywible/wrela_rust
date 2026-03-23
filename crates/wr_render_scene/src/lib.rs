#![forbid(unsafe_code)]

mod terrain_debug;

use wr_core::{CrateBoundary, CrateEntryPoint};

pub use terrain_debug::{
    CanonicalTerrainDebugScene, TerrainDebugOverlayConfig, build_terrain_debug_scene,
    canonical_hero_terrain_debug_scene,
};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_render_scene", CrateBoundary::Subsystem, false)
}
