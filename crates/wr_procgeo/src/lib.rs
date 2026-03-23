#![forbid(unsafe_code)]

mod foliage;
mod redwood;
mod terrain;

use wr_core::{CrateBoundary, CrateEntryPoint};

pub use foliage::{
    RedwoodFoliageBudget, RedwoodFoliageBuildConfig, RedwoodFoliageBuildError, RedwoodFoliageCard,
    RedwoodFoliageCluster, RedwoodFoliageLod, RedwoodFoliageLodReport,
    RedwoodFoliageMaterialPacking, RedwoodFoliageMaterialParams, RedwoodFoliageTree,
    RedwoodFoliageTreeReport, RedwoodForestFoliageReport, RedwoodForestFoliageSet,
};
pub use redwood::{
    RedwoodForestMeshReport, RedwoodForestMeshSet, RedwoodMeshAabb, RedwoodMeshBuildConfig,
    RedwoodMeshBuildError, RedwoodMeshLod, RedwoodMeshLodReport, RedwoodMeshLodTier,
    RedwoodMeshTriangle, RedwoodMeshVertex, RedwoodTreeMesh, RedwoodTreeMeshReport,
};
pub use terrain::{
    TerrainAabb, TerrainChunkCoord, TerrainChunkStats, TerrainMeshAtlas, TerrainMeshAtlasStats,
    TerrainMeshBuildConfig, TerrainMeshBuildReport, TerrainMeshChunk, TerrainTriangle,
    TerrainVertex,
};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_procgeo", CrateBoundary::Subsystem, false)
}
