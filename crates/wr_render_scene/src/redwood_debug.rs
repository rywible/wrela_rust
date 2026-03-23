use wr_physics::TerrainCollider;
use wr_procgeo::{
    RedwoodForestMeshReport, RedwoodForestMeshSet, RedwoodMeshBuildConfig, RedwoodMeshLodTier,
    RedwoodMeshTriangle, RedwoodMeshVertex, TerrainMeshAtlas, TerrainMeshBuildConfig,
};
use wr_render_api::{
    ColorRgba8, DebugTriangle, DebugVertex, ExtractedRenderScene, LinearColor, RenderGraph,
    ScenePrimitive, Vec3, debug_triangle_graph,
};
use wr_world_gen::{
    EcologicalPlacementConfig, EcologicalPlacementSet, RedwoodForestGraphConfig,
    RedwoodForestGraphSet, TerrainFieldConfig, TerrainScalarFieldSet,
};
use wr_world_seed::RootSeed;

use crate::terrain_debug::build_terrain_debug_scene;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RedwoodForestDebugOverlayConfig {
    pub lod: RedwoodMeshLodTier,
    pub max_trees: usize,
    pub show_terrain: bool,
    pub show_tree_normals: bool,
    pub show_tree_tangents: bool,
    pub vector_stride: u16,
    pub vector_length: f32,
}

impl Default for RedwoodForestDebugOverlayConfig {
    fn default() -> Self {
        Self {
            lod: RedwoodMeshLodTier::Far,
            max_trees: 32,
            show_terrain: true,
            show_tree_normals: false,
            show_tree_tangents: false,
            vector_stride: 8,
            vector_length: 2.6,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalRedwoodForestDebugScene {
    pub scene: ExtractedRenderScene,
    pub graph: RenderGraph,
    pub terrain_report: wr_procgeo::TerrainMeshBuildReport,
    pub collider_report: wr_physics::TerrainColliderBuildReport,
    pub tree_report: RedwoodForestMeshReport,
    pub rendered_tree_count: usize,
}

pub fn canonical_redwood_forest_debug_scene() -> Result<CanonicalRedwoodForestDebugScene, String> {
    let seed = RootSeed::parse_hex("0xDEADBEEF").map_err(|error| error.to_string())?;
    let fields = TerrainScalarFieldSet::generate(seed, TerrainFieldConfig::default())
        .map_err(|error| error.to_string())?;
    let placements =
        EcologicalPlacementSet::generate(seed, &fields, EcologicalPlacementConfig::default())
            .map_err(|error| error.to_string())?;
    let graphs = RedwoodForestGraphSet::generate(
        seed,
        &fields,
        &placements,
        RedwoodForestGraphConfig::default(),
    )
    .map_err(|error| error.to_string())?;
    let forest_mesh = RedwoodForestMeshSet::build(&graphs, RedwoodMeshBuildConfig::default())
        .map_err(|error| error.to_string())?;
    let terrain = TerrainMeshAtlas::build(&fields, TerrainMeshBuildConfig::default())?;
    let collider = TerrainCollider::from_mesh(&terrain);
    let rendered_tree_count = forest_mesh.trees().len().min(32);
    let scene = build_redwood_forest_debug_scene(
        &terrain,
        &forest_mesh,
        RedwoodForestDebugOverlayConfig::default(),
    );

    Ok(CanonicalRedwoodForestDebugScene {
        scene,
        graph: debug_triangle_graph(),
        terrain_report: terrain.report(),
        collider_report: collider.report(),
        tree_report: forest_mesh.report(),
        rendered_tree_count,
    })
}

pub fn build_redwood_forest_debug_scene(
    terrain: &TerrainMeshAtlas,
    forest_mesh: &RedwoodForestMeshSet,
    config: RedwoodForestDebugOverlayConfig,
) -> ExtractedRenderScene {
    let collider = TerrainCollider::from_mesh(terrain);
    let terrain_scene = if config.show_terrain {
        Some(build_terrain_debug_scene(
            terrain,
            &collider,
            crate::TerrainDebugOverlayConfig {
                show_surface: true,
                show_normals: false,
                show_tangents: false,
                show_collision_wireframe: false,
                ..crate::TerrainDebugOverlayConfig::default()
            },
        ))
    } else {
        None
    };
    let mut scene = terrain_scene
        .unwrap_or_else(|| ExtractedRenderScene::new(ColorRgba8::new(18, 24, 36, 255)));
    let projector = ForestProjector::new(terrain, forest_mesh);

    for tree in forest_mesh.trees().iter().take(config.max_trees) {
        let lod = tree.lod(config.lod);
        for triangle in lod.triangles(tree.tree_index) {
            scene.push_primitive(ScenePrimitive::DebugTriangle(surface_triangle(
                &projector, &triangle,
            )));
        }

        if config.show_tree_normals || config.show_tree_tangents {
            for (index, vertex) in lod.vertices.iter().enumerate() {
                if config.vector_stride > 1 && index % usize::from(config.vector_stride) != 0 {
                    continue;
                }

                if config.show_tree_normals {
                    scene.push_primitive(ScenePrimitive::DebugTriangle(vector_triangle(
                        &projector,
                        *vertex,
                        vertex.normal,
                        config.vector_length,
                        LinearColor::new(0.16, 0.86, 0.94, 1.0),
                    )));
                }
                if config.show_tree_tangents {
                    scene.push_primitive(ScenePrimitive::DebugTriangle(vector_triangle(
                        &projector,
                        *vertex,
                        vertex.tangent,
                        config.vector_length,
                        LinearColor::new(0.98, 0.78, 0.24, 1.0),
                    )));
                }
            }
        }
    }

    scene
}

fn surface_triangle(projector: &ForestProjector, triangle: &RedwoodMeshTriangle) -> DebugTriangle {
    let avg_height =
        (triangle.positions[0][2] + triangle.positions[1][2] + triangle.positions[2][2]) / 3.0;
    let bark = triangle.material_params.iter().map(|params| params[1]).sum::<f32>() / 3.0;
    let taper = triangle.material_params.iter().map(|params| params[0]).sum::<f32>() / 3.0;
    let height_t = projector.height_t(avg_height);
    let bark_tint =
        LinearColor::new(0.22 + 0.16 * bark, 0.14 + 0.28 * height_t, 0.08 + 0.10 * taper, 1.0);

    DebugTriangle::new([
        DebugVertex::new(projector.project(triangle.positions[0]), bark_tint),
        DebugVertex::new(projector.project(triangle.positions[1]), bark_tint),
        DebugVertex::new(projector.project(triangle.positions[2]), bark_tint),
    ])
}

fn vector_triangle(
    projector: &ForestProjector,
    vertex: RedwoodMeshVertex,
    direction: [f32; 3],
    vector_length: f32,
    color: LinearColor,
) -> DebugTriangle {
    let start = projector.project(vertex.position);
    let tip = projector.project([
        vertex.position[0] + direction[0] * vector_length,
        vertex.position[1] + direction[1] * vector_length,
        vertex.position[2] + direction[2] * vector_length,
    ]);
    let direction = [tip.x - start.x, tip.y - start.y];
    let length = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt().max(0.0001);
    let width = 0.006;
    let offset = [-(direction[1] / length) * width, (direction[0] / length) * width];

    DebugTriangle::new([
        DebugVertex::new(Vec3::new(start.x + offset[0], start.y + offset[1], start.z), color),
        DebugVertex::new(tip, color),
        DebugVertex::new(Vec3::new(start.x - offset[0], start.y - offset[1], start.z), color),
    ])
}

struct ForestProjector {
    min_x: f32,
    min_y: f32,
    width: f32,
    depth: f32,
    min_height: f32,
    max_height: f32,
}

impl ForestProjector {
    fn new(terrain: &TerrainMeshAtlas, forest_mesh: &RedwoodForestMeshSet) -> Self {
        let terrain_bounds = terrain.stats().bounds;
        let forest_bounds = forest_mesh.report().lods[RedwoodMeshLodTier::Hero.as_index()].bounds;
        Self {
            min_x: terrain_bounds.min[0].min(forest_bounds.min[0]),
            min_y: terrain_bounds.min[2].min(forest_bounds.min[1]),
            width: terrain_bounds.max[0].max(forest_bounds.max[0])
                - terrain_bounds.min[0].min(forest_bounds.min[0]),
            depth: terrain_bounds.max[2].max(forest_bounds.max[1])
                - terrain_bounds.min[2].min(forest_bounds.min[1]),
            min_height: terrain_bounds.min[1].min(forest_bounds.min[2]),
            max_height: terrain_bounds.max[1].max(forest_bounds.max[2]),
        }
    }

    fn height_t(&self, world_z: f32) -> f32 {
        let range = (self.max_height - self.min_height).max(1.0);
        ((world_z - self.min_height) / range).clamp(0.0, 1.0)
    }

    fn project(&self, world: [f32; 3]) -> Vec3 {
        let normalized_x = (((world[0] - self.min_x) / self.width.max(1.0)) * 2.0) - 1.0;
        let normalized_y = (((world[1] - self.min_y) / self.depth.max(1.0)) * 2.0) - 1.0;
        let normalized_height = (self.height_t(world[2]) * 1.55) - 0.7;
        let screen_x = normalized_x * 0.66 - normalized_y * 0.36;
        let screen_y = normalized_height * 0.82 - normalized_x * 0.08 - normalized_y * 0.18;
        let depth = normalized_y * 0.14;

        Vec3::new(screen_x, screen_y, depth)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use wr_render_wgpu::render_scene_to_png;

    use super::*;

    #[test]
    fn canonical_scene_exposes_tree_reports() {
        let scene = canonical_redwood_forest_debug_scene().expect("scene should build");

        assert!(scene.terrain_report.atlas.triangle_count > 0);
        assert!(scene.tree_report.lods[RedwoodMeshLodTier::Far.as_index()].triangle_count > 0);
        assert!(scene.rendered_tree_count > 0);
        assert!(!scene.scene.primitives.is_empty());
    }

    #[test]
    fn forest_scene_renders_to_png() {
        let scene = canonical_redwood_forest_debug_scene().expect("scene should build");
        let temp = tempdir().expect("tempdir should exist");
        let output_path = temp.path().join("redwood_forest.png");
        let outcome = render_scene_to_png(
            wr_render_api::RenderSize::new(192, 128),
            &scene.scene,
            &scene.graph,
            &output_path,
        )
        .expect("forest scene should render");

        assert!(output_path.exists());
        assert!(outcome.frame.non_empty_pixels > 0);
    }

    #[test]
    fn normal_overlay_adds_more_tree_geometry() {
        let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let fields = TerrainScalarFieldSet::generate(seed, TerrainFieldConfig::default())
            .expect("fields should generate");
        let placements =
            EcologicalPlacementSet::generate(seed, &fields, EcologicalPlacementConfig::default())
                .expect("placements should generate");
        let graphs = RedwoodForestGraphSet::generate(
            seed,
            &fields,
            &placements,
            RedwoodForestGraphConfig::default(),
        )
        .expect("graphs should generate");
        let forest_mesh = RedwoodForestMeshSet::build(&graphs, RedwoodMeshBuildConfig::default())
            .expect("mesh set should build");
        let terrain = TerrainMeshAtlas::build(&fields, TerrainMeshBuildConfig::default())
            .expect("terrain should build");

        let surface_only = build_redwood_forest_debug_scene(
            &terrain,
            &forest_mesh,
            RedwoodForestDebugOverlayConfig::default(),
        );
        let with_vectors = build_redwood_forest_debug_scene(
            &terrain,
            &forest_mesh,
            RedwoodForestDebugOverlayConfig {
                show_tree_normals: true,
                show_tree_tangents: true,
                ..RedwoodForestDebugOverlayConfig::default()
            },
        );

        assert!(with_vectors.primitives.len() > surface_only.primitives.len());
    }
}
