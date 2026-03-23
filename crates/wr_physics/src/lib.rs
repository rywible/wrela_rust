#![forbid(unsafe_code)]

mod terrain_collision;

use wr_core::{CrateBoundary, CrateEntryPoint};

pub use terrain_collision::{
    TerrainCollider, TerrainColliderBuildReport, TerrainColliderStats, TerrainRay, TerrainRayHit,
};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_physics", CrateBoundary::Subsystem, false)
}
