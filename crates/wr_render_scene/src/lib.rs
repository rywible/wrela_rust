#![forbid(unsafe_code)]

mod foliage_debug;
mod redwood_debug;
mod terrain_debug;

use wr_core::{CrateBoundary, CrateEntryPoint};

pub use foliage_debug::{
    CanonicalRedwoodFoliageDebugScene, RedwoodFoliageDebugSceneConfig,
    build_redwood_foliage_debug_scene, canonical_redwood_foliage_debug_scene,
};
pub use redwood_debug::{
    CanonicalRedwoodForestDebugScene, RedwoodForestDebugOverlayConfig,
    build_redwood_forest_debug_scene, canonical_redwood_forest_debug_scene,
};
pub use terrain_debug::{
    CanonicalTerrainDebugScene, TerrainDebugOverlayConfig, build_terrain_debug_scene,
    canonical_hero_terrain_debug_scene,
};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_render_scene", CrateBoundary::Subsystem, false)
}
