use wr_physics::TerrainCollider;
use wr_procgeo::{
    RedwoodFoliageBuildConfig, RedwoodFoliageCard, RedwoodForestFoliageReport,
    RedwoodForestFoliageSet, RedwoodForestMeshSet, RedwoodMeshBuildConfig, RedwoodMeshLodTier,
    TerrainMeshAtlas, TerrainMeshBuildConfig,
};
use wr_render_api::{
    ColorRgba8, DebugTriangle, DebugVertex, ExtractedRenderScene, FoliageCard, FoliageCardVertex,
    LinearColor, RenderGraph, ScenePrimitive, Vec3, debug_triangle_and_foliage_graph,
};
use wr_world_gen::{
    EcologicalPlacementConfig, EcologicalPlacementSet, RedwoodForestGraphConfig,
    RedwoodForestGraphSet, TerrainFieldConfig, TerrainScalarFieldSet,
};
use wr_world_seed::RootSeed;

use crate::terrain_debug::build_terrain_debug_scene;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RedwoodFoliageDebugSceneConfig {
    pub lod: RedwoodMeshLodTier,
    pub max_trees: usize,
    pub show_terrain: bool,
    pub show_trunks: bool,
}

impl Default for RedwoodFoliageDebugSceneConfig {
    fn default() -> Self {
        Self { lod: RedwoodMeshLodTier::Hero, max_trees: 12, show_terrain: true, show_trunks: true }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalRedwoodFoliageDebugScene {
    pub scene: ExtractedRenderScene,
    pub graph: RenderGraph,
    pub foliage_report: RedwoodForestFoliageReport,
    pub rendered_tree_count: usize,
}

pub fn canonical_redwood_foliage_debug_scene(
    config: RedwoodFoliageDebugSceneConfig,
) -> Result<CanonicalRedwoodFoliageDebugScene, String> {
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
    let terrain = TerrainMeshAtlas::build(&fields, TerrainMeshBuildConfig::default())?;
    let forest_mesh = RedwoodForestMeshSet::build(&graphs, RedwoodMeshBuildConfig::default())
        .map_err(|error| error.to_string())?;
    let foliage = RedwoodForestFoliageSet::build(&graphs, RedwoodFoliageBuildConfig::default())
        .map_err(|error| error.to_string())?;
    let scene = build_redwood_foliage_debug_scene(&terrain, &forest_mesh, &foliage, config);

    Ok(CanonicalRedwoodFoliageDebugScene {
        scene,
        graph: debug_triangle_and_foliage_graph(),
        foliage_report: foliage.report(),
        rendered_tree_count: foliage.trees().len().min(config.max_trees),
    })
}

pub fn build_redwood_foliage_debug_scene(
    terrain: &TerrainMeshAtlas,
    forest_mesh: &RedwoodForestMeshSet,
    foliage: &RedwoodForestFoliageSet,
    config: RedwoodFoliageDebugSceneConfig,
) -> ExtractedRenderScene {
    let collider = TerrainCollider::from_mesh(terrain);
    let mut scene = if config.show_terrain {
        build_terrain_debug_scene(
            terrain,
            &collider,
            crate::TerrainDebugOverlayConfig {
                show_surface: true,
                show_normals: false,
                show_tangents: false,
                show_collision_wireframe: false,
                ..crate::TerrainDebugOverlayConfig::default()
            },
        )
    } else {
        ExtractedRenderScene::new(ColorRgba8::new(14, 18, 26, 255))
    };
    let projector = ForestProjector::new(terrain, forest_mesh, foliage);

    if config.show_trunks {
        for tree in forest_mesh.trees().iter().take(config.max_trees) {
            for triangle in tree.lod(config.lod).triangles(tree.tree_index) {
                scene.push_primitive(ScenePrimitive::DebugTriangle(surface_triangle(
                    &projector, &triangle,
                )));
            }
        }
    }

    for tree in foliage.trees().iter().take(config.max_trees) {
        for cluster in &tree.lod(config.lod).clusters {
            for card in &cluster.cards {
                scene.push_primitive(ScenePrimitive::FoliageCard(project_card(&projector, *card)));
            }
        }
    }

    scene
}

fn project_card(projector: &ForestProjector, card: RedwoodFoliageCard) -> FoliageCard {
    let center = projector.project(card.center);
    let axis_u = projector.project_vector(card.axis_u, card.half_extents_m[0]);
    let axis_v = projector.project_vector(card.axis_v, card.half_extents_m[1]);
    let normal = projector.project_direction(card.normal);
    let corners = [
        add_vec3(add_vec3(center, scale_vec3(axis_u, -1.0)), scale_vec3(axis_v, -1.0)),
        add_vec3(add_vec3(center, axis_u), scale_vec3(axis_v, -1.0)),
        add_vec3(add_vec3(center, axis_u), axis_v),
        add_vec3(add_vec3(center, scale_vec3(axis_u, -1.0)), axis_v),
    ];

    FoliageCard::new([
        FoliageCardVertex::new(corners[0], [0.0, 0.0], normal, card.packed_material_params.words),
        FoliageCardVertex::new(corners[1], [1.0, 0.0], normal, card.packed_material_params.words),
        FoliageCardVertex::new(corners[2], [1.0, 1.0], normal, card.packed_material_params.words),
        FoliageCardVertex::new(corners[3], [0.0, 1.0], normal, card.packed_material_params.words),
    ])
}

fn surface_triangle(
    projector: &ForestProjector,
    triangle: &wr_procgeo::RedwoodMeshTriangle,
) -> DebugTriangle {
    let avg_height =
        (triangle.positions[0][2] + triangle.positions[1][2] + triangle.positions[2][2]) / 3.0;
    let bark = triangle.material_params.iter().map(|params| params[1]).sum::<f32>() / 3.0;
    let taper = triangle.material_params.iter().map(|params| params[0]).sum::<f32>() / 3.0;
    let height_t = projector.height_t(avg_height);
    let bark_tint =
        LinearColor::new(0.20 + 0.12 * bark, 0.13 + 0.18 * height_t, 0.08 + 0.08 * taper, 1.0);

    DebugTriangle::new([
        DebugVertex::new(projector.project(triangle.positions[0]), bark_tint),
        DebugVertex::new(projector.project(triangle.positions[1]), bark_tint),
        DebugVertex::new(projector.project(triangle.positions[2]), bark_tint),
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
    fn new(
        terrain: &TerrainMeshAtlas,
        forest_mesh: &RedwoodForestMeshSet,
        foliage: &RedwoodForestFoliageSet,
    ) -> Self {
        let terrain_bounds = terrain.stats().bounds;
        let mesh_bounds = forest_mesh.report().lods[RedwoodMeshLodTier::Hero.as_index()].bounds;
        let foliage_bounds = foliage.report().lods[RedwoodMeshLodTier::Hero.as_index()].bounds;
        // Terrain meshes are stored as [x, height, depth] while tree and foliage bounds report
        // [x, depth, height]. Keep the axis swap local and documented in this projector.
        Self {
            min_x: terrain_bounds.min[0].min(mesh_bounds.min[0]).min(foliage_bounds.min[0]),
            min_y: terrain_bounds.min[2].min(mesh_bounds.min[1]).min(foliage_bounds.min[1]),
            width: terrain_bounds.max[0].max(mesh_bounds.max[0]).max(foliage_bounds.max[0])
                - terrain_bounds.min[0].min(mesh_bounds.min[0]).min(foliage_bounds.min[0]),
            depth: terrain_bounds.max[2].max(mesh_bounds.max[1]).max(foliage_bounds.max[1])
                - terrain_bounds.min[2].min(mesh_bounds.min[1]).min(foliage_bounds.min[1]),
            min_height: terrain_bounds.min[1].min(mesh_bounds.min[2]).min(foliage_bounds.min[2]),
            max_height: terrain_bounds.max[1].max(mesh_bounds.max[2]).max(foliage_bounds.max[2]),
        }
    }

    fn height_t(&self, world_z: f32) -> f32 {
        let range = (self.max_height - self.min_height).max(1.0);
        ((world_z - self.min_height) / range).clamp(0.0, 1.0)
    }

    fn project(&self, world: [f32; 3]) -> Vec3 {
        let normalized_x = (((world[0] - self.min_x) / self.width.max(1.0)) * 2.0) - 1.0;
        let normalized_y = (((world[1] - self.min_y) / self.depth.max(1.0)) * 2.0) - 1.0;
        let normalized_height = (self.height_t(world[2]) * 1.55) - 0.72;
        let screen_x = normalized_x * 0.64 - normalized_y * 0.34;
        let screen_y = normalized_height * 0.90 - normalized_x * 0.04 - normalized_y * 0.16;
        let depth = normalized_y * 0.12;
        Vec3::new(screen_x, screen_y, depth)
    }

    fn project_direction(&self, direction: [f32; 3]) -> Vec3 {
        let start = self.project([0.0, 0.0, 0.0]);
        let tip = self.project(direction);
        Vec3::new(tip.x - start.x, tip.y - start.y, tip.z - start.z)
    }

    fn project_vector(&self, direction: [f32; 3], scale: f32) -> Vec3 {
        let projected = self.project_direction([
            direction[0] * scale,
            direction[1] * scale,
            direction[2] * scale,
        ]);
        let length = ((projected.x * projected.x) + (projected.y * projected.y)).sqrt().max(0.0001);
        let normalized = Vec3::new(projected.x / length, projected.y / length, projected.z);
        Vec3::new(normalized.x * scale * 0.07, normalized.y * scale * 0.07, normalized.z * 0.02)
    }
}

fn add_vec3(lhs: Vec3, rhs: Vec3) -> Vec3 {
    Vec3::new(lhs.x + rhs.x, lhs.y + rhs.y, lhs.z + rhs.z)
}

fn scale_vec3(vector: Vec3, scalar: f32) -> Vec3 {
    Vec3::new(vector.x * scalar, vector.y * scalar, vector.z * scalar)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use wr_render_wgpu::render_scene_to_png;

    use super::*;

    #[test]
    fn canonical_foliage_scene_reports_budget_and_density() {
        let scene =
            canonical_redwood_foliage_debug_scene(RedwoodFoliageDebugSceneConfig::default())
                .expect("scene should build");

        assert!(scene.foliage_report.total_tip_clusters > 0);
        assert!(scene.foliage_report.lods[RedwoodMeshLodTier::Hero.as_index()].card_count > 0);
        assert!(scene.foliage_report.within_budget);
        assert!(scene.rendered_tree_count > 0);
    }

    #[test]
    fn near_canopy_scene_renders_to_png() {
        let scene = canonical_redwood_foliage_debug_scene(RedwoodFoliageDebugSceneConfig {
            lod: RedwoodMeshLodTier::Hero,
            max_trees: 8,
            show_terrain: true,
            show_trunks: true,
        })
        .expect("scene should build");
        let temp = tempdir().expect("tempdir should exist");
        let output_path = temp.path().join("foliage_near.png");
        let outcome = render_scene_to_png(
            wr_render_api::RenderSize::new(256, 192),
            &scene.scene,
            &scene.graph,
            &output_path,
        )
        .expect("render should succeed");

        assert!(output_path.exists());
        assert!(outcome.frame.non_empty_pixels > 12_000);
    }

    #[test]
    fn far_canopy_scene_renders_to_png() {
        let scene = canonical_redwood_foliage_debug_scene(RedwoodFoliageDebugSceneConfig {
            lod: RedwoodMeshLodTier::Far,
            max_trees: 48,
            show_terrain: true,
            show_trunks: false,
        })
        .expect("scene should build");
        let temp = tempdir().expect("tempdir should exist");
        let output_path = temp.path().join("foliage_far.png");
        let outcome = render_scene_to_png(
            wr_render_api::RenderSize::new(256, 192),
            &scene.scene,
            &scene.graph,
            &output_path,
        )
        .expect("render should succeed");

        assert!(output_path.exists());
        assert!(outcome.frame.non_empty_pixels > 9_000);
    }
}
