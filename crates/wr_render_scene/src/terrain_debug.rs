use wr_physics::TerrainCollider;
use wr_procgeo::{TerrainMeshAtlas, TerrainMeshBuildConfig, TerrainTriangle, TerrainVertex};
use wr_render_api::{
    ColorRgba8, DebugTriangle, DebugVertex, ExtractedRenderScene, LinearColor, RenderGraph,
    ScenePrimitive, Vec3, debug_triangle_graph,
};
use wr_world_gen::{TerrainFieldConfig, TerrainScalarFieldSet};
use wr_world_seed::RootSeed;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainDebugOverlayConfig {
    pub show_surface: bool,
    pub show_normals: bool,
    pub show_tangents: bool,
    pub show_collision_wireframe: bool,
    pub vector_stride: u16,
    pub vector_length: f32,
}

impl Default for TerrainDebugOverlayConfig {
    fn default() -> Self {
        Self {
            show_surface: true,
            show_normals: false,
            show_tangents: false,
            show_collision_wireframe: false,
            vector_stride: 12,
            vector_length: 10.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalTerrainDebugScene {
    pub scene: ExtractedRenderScene,
    pub graph: RenderGraph,
    pub mesh_report: wr_procgeo::TerrainMeshBuildReport,
    pub collider_report: wr_physics::TerrainColliderBuildReport,
}

pub fn canonical_hero_terrain_debug_scene() -> Result<CanonicalTerrainDebugScene, String> {
    let seed = RootSeed::parse_hex("0xDEADBEEF").map_err(|error| error.to_string())?;
    let fields = TerrainScalarFieldSet::generate(seed, TerrainFieldConfig::default())
        .map_err(|error| error.to_string())?;
    let mesh = TerrainMeshAtlas::build(&fields, TerrainMeshBuildConfig::default())?;
    let collider = TerrainCollider::from_mesh(&mesh);
    let scene = build_terrain_debug_scene(
        &mesh,
        &collider,
        TerrainDebugOverlayConfig {
            show_surface: true,
            show_normals: false,
            show_tangents: false,
            show_collision_wireframe: true,
            ..TerrainDebugOverlayConfig::default()
        },
    );

    Ok(CanonicalTerrainDebugScene {
        scene,
        graph: debug_triangle_graph(),
        mesh_report: mesh.report(),
        collider_report: collider.report(),
    })
}

pub fn build_terrain_debug_scene(
    mesh: &TerrainMeshAtlas,
    collider: &TerrainCollider,
    config: TerrainDebugOverlayConfig,
) -> ExtractedRenderScene {
    let mut scene = ExtractedRenderScene::new(ColorRgba8::new(18, 28, 44, 255));
    let projector = TerrainProjector::new(mesh);

    if config.show_surface {
        for triangle in mesh.triangles() {
            scene.push_primitive(ScenePrimitive::DebugTriangle(surface_triangle(
                &projector, &triangle, mesh,
            )));
        }
    }

    if config.show_normals || config.show_tangents {
        for chunk in mesh.chunks() {
            for vertex in &chunk.vertices {
                if config.vector_stride > 1
                    && (u32::from(vertex.grid_column) + u32::from(vertex.grid_row))
                        % u32::from(config.vector_stride)
                        != 0
                {
                    continue;
                }

                if config.show_normals {
                    scene.push_primitive(ScenePrimitive::DebugTriangle(vector_triangle(
                        &projector,
                        vertex,
                        vertex.normal,
                        config.vector_length,
                        LinearColor::new(0.16, 0.85, 0.92, 1.0),
                    )));
                }
                if config.show_tangents {
                    scene.push_primitive(ScenePrimitive::DebugTriangle(vector_triangle(
                        &projector,
                        vertex,
                        vertex.tangent,
                        config.vector_length,
                        LinearColor::new(0.96, 0.72, 0.24, 1.0),
                    )));
                }
            }
        }
    }

    if config.show_collision_wireframe {
        for triangle in collider.triangles() {
            for edge in [[0, 1], [1, 2], [2, 0]] {
                for wire_triangle in edge_ribbon_triangles(
                    &projector,
                    triangle.positions[edge[0]],
                    triangle.positions[edge[1]],
                    0.006,
                    LinearColor::new(0.96, 0.22, 0.28, 1.0),
                ) {
                    scene.push_primitive(ScenePrimitive::DebugTriangle(wire_triangle));
                }
            }
        }
    }

    scene
}

fn surface_triangle(
    projector: &TerrainProjector,
    triangle: &TerrainTriangle,
    mesh: &TerrainMeshAtlas,
) -> DebugTriangle {
    let avg_height =
        (triangle.positions[0][1] + triangle.positions[1][1] + triangle.positions[2][1]) / 3.0;
    let height_t = inverse_lerp(
        mesh.stats().min_height,
        mesh.stats().max_height.max(mesh.stats().min_height + 1.0),
        avg_height,
    );
    let brightness = 0.35 + (0.45 * height_t);
    let tint = LinearColor::new(0.16 + 0.18 * height_t, brightness, 0.22, 1.0);

    DebugTriangle::new([
        DebugVertex::new(projector.project(triangle.positions[0]), tint),
        DebugVertex::new(projector.project(triangle.positions[1]), tint),
        DebugVertex::new(projector.project(triangle.positions[2]), tint),
    ])
}

fn vector_triangle(
    projector: &TerrainProjector,
    vertex: &TerrainVertex,
    direction: [f32; 3],
    vector_length: f32,
    color: LinearColor,
) -> DebugTriangle {
    let start = projector.project(vertex.position);
    let tip_position = [
        vertex.position[0] + direction[0] * vector_length,
        vertex.position[1] + direction[1] * vector_length,
        vertex.position[2] + direction[2] * vector_length,
    ];
    let tip = projector.project(tip_position);
    let screen_direction = [tip.x - start.x, tip.y - start.y];
    let length = (screen_direction[0] * screen_direction[0]
        + screen_direction[1] * screen_direction[1])
        .sqrt()
        .max(0.0001);
    let width = 0.010;
    let offset = [-(screen_direction[1] / length) * width, (screen_direction[0] / length) * width];

    DebugTriangle::new([
        DebugVertex::new(Vec3::new(start.x + offset[0], start.y + offset[1], start.z), color),
        DebugVertex::new(tip, color),
        DebugVertex::new(Vec3::new(start.x - offset[0], start.y - offset[1], start.z), color),
    ])
}

fn edge_ribbon_triangles(
    projector: &TerrainProjector,
    start: [f32; 3],
    end: [f32; 3],
    width: f32,
    color: LinearColor,
) -> [DebugTriangle; 2] {
    let start = projector.project(start);
    let end = projector.project(end);
    let direction = [end.x - start.x, end.y - start.y];
    let length = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt().max(0.0001);
    let offset = [-(direction[1] / length) * width, (direction[0] / length) * width];
    let a = Vec3::new(start.x + offset[0], start.y + offset[1], start.z);
    let b = Vec3::new(start.x - offset[0], start.y - offset[1], start.z);
    let c = Vec3::new(end.x + offset[0], end.y + offset[1], end.z);
    let d = Vec3::new(end.x - offset[0], end.y - offset[1], end.z);

    [
        DebugTriangle::new([
            DebugVertex::new(a, color),
            DebugVertex::new(b, color),
            DebugVertex::new(c, color),
        ]),
        DebugTriangle::new([
            DebugVertex::new(c, color),
            DebugVertex::new(b, color),
            DebugVertex::new(d, color),
        ]),
    ]
}

struct TerrainProjector {
    min_height: f32,
    max_height: f32,
    width: f32,
    depth: f32,
}

impl TerrainProjector {
    fn new(mesh: &TerrainMeshAtlas) -> Self {
        Self {
            min_height: mesh.stats().min_height,
            max_height: mesh.stats().max_height,
            width: mesh.stats().bounds.max[0] - mesh.stats().bounds.min[0],
            depth: mesh.stats().bounds.max[2] - mesh.stats().bounds.min[2],
        }
    }

    fn project(&self, world: [f32; 3]) -> Vec3 {
        let normalized_x = ((world[0] / self.width.max(1.0)) * 2.0) - 1.0;
        let normalized_z = ((world[2] / self.depth.max(1.0)) * 2.0) - 1.0;
        let normalized_height =
            (inverse_lerp(self.min_height, self.max_height.max(self.min_height + 1.0), world[1])
                * 1.2)
                - 0.45;
        let screen_x = normalized_x * 0.62 - normalized_z * 0.34;
        let screen_y = normalized_height * 0.78 - normalized_x * 0.12 - normalized_z * 0.16;
        let depth = (normalized_z * 0.15) + 0.1;

        Vec3::new(screen_x, screen_y, depth)
    }
}

fn inverse_lerp(start: f32, end: f32, value: f32) -> f32 {
    let range = end - start;
    if range.abs() <= f32::EPSILON { 0.0 } else { ((value - start) / range).clamp(0.0, 1.0) }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use wr_render_wgpu::render_scene_to_png;

    use super::*;

    #[test]
    fn canonical_scene_exposes_mesh_and_collider_reports() {
        let scene = canonical_hero_terrain_debug_scene().expect("canonical scene should build");

        assert!(scene.mesh_report.atlas.triangle_count > 0);
        assert_eq!(
            scene.mesh_report.atlas.triangle_count,
            scene.collider_report.stats.triangle_count
        );
        assert!(!scene.scene.primitives.is_empty());
    }

    #[test]
    fn overlay_scene_renders_to_png() {
        let scene = canonical_hero_terrain_debug_scene().expect("canonical scene should build");
        let temp = tempdir().expect("tempdir should exist");
        let output_path = temp.path().join("terrain.png");
        let outcome = render_scene_to_png(
            wr_render_api::RenderSize::new(160, 120),
            &scene.scene,
            &scene.graph,
            &output_path,
        )
        .expect("terrain scene should render");

        assert!(output_path.exists());
        assert!(outcome.frame.non_empty_pixels > 0);
    }

    #[test]
    fn collision_wireframe_overlay_adds_more_geometry() {
        let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let fields = TerrainScalarFieldSet::generate(seed, TerrainFieldConfig::default())
            .expect("field set should build");
        let mesh = TerrainMeshAtlas::build(&fields, TerrainMeshBuildConfig::default())
            .expect("mesh should build");
        let collider = TerrainCollider::from_mesh(&mesh);
        let surface_only =
            build_terrain_debug_scene(&mesh, &collider, TerrainDebugOverlayConfig::default());
        let with_overlays = build_terrain_debug_scene(
            &mesh,
            &collider,
            TerrainDebugOverlayConfig {
                show_surface: true,
                show_normals: true,
                show_tangents: true,
                show_collision_wireframe: true,
                vector_stride: 16,
                vector_length: 8.0,
            },
        );

        assert!(with_overlays.primitives.len() > surface_only.primitives.len());
    }
}
