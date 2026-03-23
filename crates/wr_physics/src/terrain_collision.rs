use serde::{Deserialize, Serialize};
use wr_math::Vec2;
use wr_procgeo::{TerrainAabb, TerrainChunkCoord, TerrainMeshAtlas, TerrainTriangle};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainRay {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
}

impl TerrainRay {
    pub const fn new(origin: [f32; 3], direction: [f32; 3]) -> Self {
        Self { origin, direction }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainRayHit {
    pub chunk: TerrainChunkCoord,
    pub distance: f32,
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainColliderStats {
    pub chunk_count: u32,
    pub triangle_count: u32,
    pub min_height: f32,
    pub max_height: f32,
    pub bounds: TerrainAabb,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainColliderBuildReport {
    pub seed_hex: String,
    pub stats: TerrainColliderStats,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerrainCollider {
    seed_hex: String,
    triangles: Vec<TerrainTriangle>,
    stats: TerrainColliderStats,
}

impl TerrainCollider {
    pub fn from_mesh(mesh: &TerrainMeshAtlas) -> Self {
        Self {
            seed_hex: mesh.seed_hex().to_owned(),
            triangles: mesh.triangles(),
            stats: TerrainColliderStats {
                chunk_count: mesh.stats().chunk_count,
                triangle_count: mesh.stats().triangle_count,
                min_height: mesh.stats().min_height,
                max_height: mesh.stats().max_height,
                bounds: mesh.stats().bounds,
            },
        }
    }

    pub fn seed_hex(&self) -> &str {
        &self.seed_hex
    }

    pub fn triangles(&self) -> &[TerrainTriangle] {
        &self.triangles
    }

    pub fn stats(&self) -> TerrainColliderStats {
        self.stats
    }

    pub fn report(&self) -> TerrainColliderBuildReport {
        TerrainColliderBuildReport { seed_hex: self.seed_hex.clone(), stats: self.stats }
    }

    pub fn raycast(&self, ray: TerrainRay, max_distance: f32) -> Option<TerrainRayHit> {
        let mut best_hit = None;

        for triangle in &self.triangles {
            if let Some(distance) =
                intersect_triangle(ray.origin, ray.direction, triangle.positions, max_distance)
            {
                let replace = best_hit
                    .as_ref()
                    .is_none_or(|current: &TerrainRayHit| distance < current.distance);
                if replace {
                    let position = [
                        ray.origin[0] + ray.direction[0] * distance,
                        ray.origin[1] + ray.direction[1] * distance,
                        ray.origin[2] + ray.direction[2] * distance,
                    ];
                    best_hit = Some(TerrainRayHit {
                        chunk: triangle.chunk,
                        distance,
                        position,
                        normal: face_normal(triangle.positions),
                    });
                }
            }
        }

        best_hit
    }

    pub fn sample_height(&self, point: Vec2) -> Option<f32> {
        let ray =
            TerrainRay::new([point.x, self.stats.max_height + 16.0, point.y], [0.0, -1.0, 0.0]);
        self.raycast(ray, self.stats.max_height - self.stats.min_height + 64.0)
            .map(|hit| hit.position[1])
    }
}

fn intersect_triangle(
    origin: [f32; 3],
    direction: [f32; 3],
    triangle: [[f32; 3]; 3],
    max_distance: f32,
) -> Option<f32> {
    let edge_a = subtract3(triangle[1], triangle[0]);
    let edge_b = subtract3(triangle[2], triangle[0]);
    let pvec = cross3(direction, edge_b);
    let determinant = dot3(edge_a, pvec);
    if determinant.abs() <= 1.0e-6 {
        return None;
    }

    let inv_det = 1.0 / determinant;
    let tvec = subtract3(origin, triangle[0]);
    let u = dot3(tvec, pvec) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let qvec = cross3(tvec, edge_a);
    let v = dot3(direction, qvec) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let distance = dot3(edge_b, qvec) * inv_det;
    if distance < 0.0 || distance > max_distance {
        return None;
    }

    Some(distance)
}

fn face_normal(triangle: [[f32; 3]; 3]) -> [f32; 3] {
    normalize3(cross3(subtract3(triangle[1], triangle[0]), subtract3(triangle[2], triangle[0])))
}

fn subtract3(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn dot3(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross3(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn normalize3(vector: [f32; 3]) -> [f32; 3] {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if length <= f32::EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        [vector[0] / length, vector[1] / length, vector[2] / length]
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_json_snapshot;
    use wr_procgeo::{TerrainMeshAtlas, TerrainMeshBuildConfig};
    use wr_world_gen::{TerrainFieldConfig, TerrainScalarFieldSet};
    use wr_world_seed::RootSeed;

    use super::*;

    fn canonical_collider() -> TerrainCollider {
        let fields = TerrainScalarFieldSet::generate(
            RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse"),
            TerrainFieldConfig::default(),
        )
        .expect("field set should build");
        let mesh = TerrainMeshAtlas::build(&fields, TerrainMeshBuildConfig::default())
            .expect("mesh should build");
        TerrainCollider::from_mesh(&mesh)
    }

    #[test]
    fn collider_report_matches_snapshot() {
        assert_json_snapshot!("terrain_collider_report", canonical_collider().report());
    }

    #[test]
    fn downward_raycast_hits_the_terrain_surface() {
        let collider = canonical_collider();
        let hit = collider
            .raycast(TerrainRay::new([256.0, 192.0, 256.0], [0.0, -1.0, 0.0]), 256.0)
            .expect("downward ray should hit");

        assert!(hit.position[1] >= collider.stats().min_height);
        assert!(hit.position[1] <= collider.stats().max_height);
        assert!(hit.normal[1] > 0.0);
    }

    #[test]
    fn sample_height_uses_same_surface_as_raycast() {
        let collider = canonical_collider();
        let point = Vec2::new(128.0, 384.0);
        let height = collider.sample_height(point).expect("sample height should exist");
        let ray_hit = collider
            .raycast(
                TerrainRay::new(
                    [point.x, collider.stats().max_height + 16.0, point.y],
                    [0.0, -1.0, 0.0],
                ),
                collider.stats().max_height - collider.stats().min_height + 64.0,
            )
            .expect("downward ray should hit");

        assert!((height - ray_hit.position[1]).abs() <= 0.001);
    }
}
