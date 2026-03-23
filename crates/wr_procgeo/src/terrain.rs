use serde::{Deserialize, Serialize};
use wr_world_gen::{TerrainFieldKind, TerrainScalarFieldSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TerrainChunkCoord {
    pub x: u16,
    pub y: u16,
}

impl TerrainChunkCoord {
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainVertex {
    pub grid_column: u16,
    pub grid_row: u16,
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainTriangle {
    pub chunk: TerrainChunkCoord,
    pub indices: [u32; 3],
    pub positions: [[f32; 3]; 3],
    pub normals: [[f32; 3]; 3],
    pub tangents: [[f32; 3]; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainAabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl TerrainAabb {
    const fn empty() -> Self {
        Self {
            min: [f32::INFINITY, f32::INFINITY, f32::INFINITY],
            max: [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
        }
    }

    fn include_position(&mut self, position: [f32; 3]) {
        self.min[0] = self.min[0].min(position[0]);
        self.min[1] = self.min[1].min(position[1]);
        self.min[2] = self.min[2].min(position[2]);
        self.max[0] = self.max[0].max(position[0]);
        self.max[1] = self.max[1].max(position[1]);
        self.max[2] = self.max[2].max(position[2]);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainChunkStats {
    pub chunk: TerrainChunkCoord,
    pub quads_wide: u16,
    pub quads_tall: u16,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub min_height: f32,
    pub max_height: f32,
    pub bounds: TerrainAabb,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainMeshAtlasStats {
    pub terrain_resolution: u16,
    pub chunk_resolution: u16,
    pub chunk_columns: u16,
    pub chunk_rows: u16,
    pub chunk_count: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub min_height: f32,
    pub max_height: f32,
    pub bounds: TerrainAabb,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainMeshBuildReport {
    pub seed_hex: String,
    pub atlas: TerrainMeshAtlasStats,
    pub chunks: Vec<TerrainChunkStats>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerrainMeshChunk {
    pub chunk: TerrainChunkCoord,
    pub quads_wide: u16,
    pub quads_tall: u16,
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
    pub stats: TerrainChunkStats,
}

impl TerrainMeshChunk {
    pub fn triangles(&self) -> Vec<TerrainTriangle> {
        self.indices
            .chunks_exact(3)
            .map(|indices| {
                let left = self.vertices[indices[0] as usize];
                let middle = self.vertices[indices[1] as usize];
                let right = self.vertices[indices[2] as usize];

                TerrainTriangle {
                    chunk: self.chunk,
                    indices: [indices[0], indices[1], indices[2]],
                    positions: [left.position, middle.position, right.position],
                    normals: [left.normal, middle.normal, right.normal],
                    tangents: [left.tangent, middle.tangent, right.tangent],
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerrainMeshAtlas {
    seed_hex: String,
    chunk_resolution: u16,
    chunks: Vec<TerrainMeshChunk>,
    stats: TerrainMeshAtlasStats,
}

impl TerrainMeshAtlas {
    pub fn build(
        fields: &TerrainScalarFieldSet,
        config: TerrainMeshBuildConfig,
    ) -> Result<Self, String> {
        let config = config.validate(fields)?;
        let terrain_resolution = fields.cache_resolution();
        let grid_step_x = fields.config().width_m / f32::from(terrain_resolution.saturating_sub(1));
        let grid_step_z =
            fields.config().height_m / f32::from(terrain_resolution.saturating_sub(1));
        let total_quads = terrain_resolution.saturating_sub(1);
        let chunk_columns = total_quads.div_ceil(config.chunk_resolution);
        let chunk_rows = total_quads.div_ceil(config.chunk_resolution);
        let mut global_vertices = Vec::with_capacity(usize::from(terrain_resolution).pow(2));

        for row in 0..terrain_resolution {
            for column in 0..terrain_resolution {
                let point = fields.grid_point(column, row);
                let height = fields.sample_field(TerrainFieldKind::Height, point);
                let left = fields.sample_field(
                    TerrainFieldKind::Height,
                    fields.grid_point(column.saturating_sub(1), row),
                );
                let right = fields.sample_field(
                    TerrainFieldKind::Height,
                    fields.grid_point((column + 1).min(terrain_resolution - 1), row),
                );
                let down = fields.sample_field(
                    TerrainFieldKind::Height,
                    fields.grid_point(column, row.saturating_sub(1)),
                );
                let up = fields.sample_field(
                    TerrainFieldKind::Height,
                    fields.grid_point(column, (row + 1).min(terrain_resolution - 1)),
                );
                let tangent = normalize3([2.0 * grid_step_x, right - left, 0.0]);
                let bitangent = normalize3([0.0, up - down, 2.0 * grid_step_z]);
                let normal = normalize3(cross3(bitangent, tangent));

                global_vertices.push(TerrainVertex {
                    grid_column: column,
                    grid_row: row,
                    position: [point.x, height, point.y],
                    normal,
                    tangent,
                });
            }
        }

        let mut chunks = Vec::with_capacity(usize::from(chunk_columns) * usize::from(chunk_rows));
        let mut atlas_bounds = TerrainAabb::empty();
        let mut atlas_min_height = f32::INFINITY;
        let mut atlas_max_height = f32::NEG_INFINITY;
        let mut atlas_vertex_count = 0_u32;
        let mut atlas_triangle_count = 0_u32;

        for chunk_y in 0..chunk_rows {
            for chunk_x in 0..chunk_columns {
                let start_quad_x = chunk_x * config.chunk_resolution;
                let start_quad_y = chunk_y * config.chunk_resolution;
                let quads_wide = (total_quads - start_quad_x).min(config.chunk_resolution);
                let quads_tall = (total_quads - start_quad_y).min(config.chunk_resolution);
                let local_width = quads_wide + 1;
                let local_height = quads_tall + 1;
                let mut vertices =
                    Vec::with_capacity(usize::from(local_width) * usize::from(local_height));
                let mut bounds = TerrainAabb::empty();
                let mut min_height = f32::INFINITY;
                let mut max_height = f32::NEG_INFINITY;

                for local_row in 0..local_height {
                    for local_column in 0..local_width {
                        let global_column = start_quad_x + local_column;
                        let global_row = start_quad_y + local_row;
                        let vertex = global_vertices[usize::from(global_row)
                            * usize::from(terrain_resolution)
                            + usize::from(global_column)];
                        bounds.include_position(vertex.position);
                        min_height = min_height.min(vertex.position[1]);
                        max_height = max_height.max(vertex.position[1]);
                        vertices.push(vertex);
                    }
                }

                let mut indices =
                    Vec::with_capacity(usize::from(quads_wide) * usize::from(quads_tall) * 6);
                for local_row in 0..quads_tall {
                    for local_column in 0..quads_wide {
                        let global_column = start_quad_x + local_column;
                        let global_row = start_quad_y + local_row;
                        let top_left = usize::from(local_row) * usize::from(local_width)
                            + usize::from(local_column);
                        let top_right = top_left + 1;
                        let bottom_left = top_left + usize::from(local_width);
                        let bottom_right = bottom_left + 1;
                        let diagonal_flips =
                            (u32::from(global_column) + u32::from(global_row)) % 2 == 1;
                        let quad_indices = if diagonal_flips {
                            [
                                top_left as u32,
                                bottom_left as u32,
                                bottom_right as u32,
                                top_left as u32,
                                bottom_right as u32,
                                top_right as u32,
                            ]
                        } else {
                            [
                                top_left as u32,
                                bottom_left as u32,
                                top_right as u32,
                                top_right as u32,
                                bottom_left as u32,
                                bottom_right as u32,
                            ]
                        };
                        indices.extend_from_slice(&quad_indices);
                    }
                }

                let stats = TerrainChunkStats {
                    chunk: TerrainChunkCoord::new(chunk_x, chunk_y),
                    quads_wide,
                    quads_tall,
                    vertex_count: vertices.len() as u32,
                    triangle_count: (indices.len() / 3) as u32,
                    min_height,
                    max_height,
                    bounds,
                };
                atlas_bounds.include_position(bounds.min);
                atlas_bounds.include_position(bounds.max);
                atlas_min_height = atlas_min_height.min(min_height);
                atlas_max_height = atlas_max_height.max(max_height);
                atlas_vertex_count += stats.vertex_count;
                atlas_triangle_count += stats.triangle_count;

                chunks.push(TerrainMeshChunk {
                    chunk: TerrainChunkCoord::new(chunk_x, chunk_y),
                    quads_wide,
                    quads_tall,
                    vertices,
                    indices,
                    stats,
                });
            }
        }

        Ok(Self {
            seed_hex: fields.seed_hex().to_owned(),
            chunk_resolution: config.chunk_resolution,
            stats: TerrainMeshAtlasStats {
                terrain_resolution,
                chunk_resolution: config.chunk_resolution,
                chunk_columns,
                chunk_rows,
                chunk_count: chunks.len() as u32,
                vertex_count: atlas_vertex_count,
                triangle_count: atlas_triangle_count,
                min_height: atlas_min_height,
                max_height: atlas_max_height,
                bounds: atlas_bounds,
            },
            chunks,
        })
    }

    pub fn seed_hex(&self) -> &str {
        &self.seed_hex
    }

    pub fn chunk_resolution(&self) -> u16 {
        self.chunk_resolution
    }

    pub fn chunks(&self) -> &[TerrainMeshChunk] {
        &self.chunks
    }

    pub fn stats(&self) -> TerrainMeshAtlasStats {
        self.stats
    }

    pub fn report(&self) -> TerrainMeshBuildReport {
        TerrainMeshBuildReport {
            seed_hex: self.seed_hex.clone(),
            atlas: self.stats,
            chunks: self.chunks.iter().map(|chunk| chunk.stats).collect(),
        }
    }

    pub fn chunk(&self, coord: TerrainChunkCoord) -> Option<&TerrainMeshChunk> {
        self.chunks.iter().find(|chunk| chunk.chunk == coord)
    }

    pub fn triangles(&self) -> Vec<TerrainTriangle> {
        self.chunks.iter().flat_map(TerrainMeshChunk::triangles).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainMeshBuildConfig {
    pub chunk_resolution: u16,
}

impl TerrainMeshBuildConfig {
    pub const fn new(chunk_resolution: u16) -> Self {
        Self { chunk_resolution }
    }

    pub fn validate(self, fields: &TerrainScalarFieldSet) -> Result<Self, String> {
        if self.chunk_resolution == 0 {
            return Err(String::from("chunk_resolution must be greater than zero"));
        }
        if self.chunk_resolution >= fields.cache_resolution() {
            return Err(format!(
                "chunk_resolution {} must be smaller than terrain resolution {}",
                self.chunk_resolution,
                fields.cache_resolution()
            ));
        }
        Ok(self)
    }
}

impl Default for TerrainMeshBuildConfig {
    fn default() -> Self {
        Self { chunk_resolution: 16 }
    }
}

fn normalize3(vector: [f32; 3]) -> [f32; 3] {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if length <= f32::EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        [vector[0] / length, vector[1] / length, vector[2] / length]
    }
}

fn cross3(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

#[cfg(test)]
mod tests {
    use insta::assert_json_snapshot;
    use proptest::prelude::*;
    use wr_world_gen::{TerrainFieldConfig, TerrainScalarFieldSet};
    use wr_world_seed::RootSeed;

    use super::*;

    fn canonical_fields() -> TerrainScalarFieldSet {
        TerrainScalarFieldSet::generate(
            RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse"),
            TerrainFieldConfig::default(),
        )
        .expect("field set should build")
    }

    #[test]
    fn mesh_report_matches_snapshot() {
        let atlas = TerrainMeshAtlas::build(&canonical_fields(), TerrainMeshBuildConfig::default())
            .expect("terrain mesh atlas should build");

        assert_json_snapshot!("terrain_mesh_report", atlas.report());
    }

    #[test]
    fn adjacent_chunks_share_identical_boundary_vertices() {
        let atlas = TerrainMeshAtlas::build(&canonical_fields(), TerrainMeshBuildConfig::default())
            .expect("terrain mesh atlas should build");
        let left = atlas.chunk(TerrainChunkCoord::new(0, 0)).expect("left chunk should exist");
        let right = atlas.chunk(TerrainChunkCoord::new(1, 0)).expect("right chunk should exist");

        let left_edge = left
            .vertices
            .iter()
            .filter(|vertex| {
                vertex.grid_column
                    == left.vertices.iter().map(|v| v.grid_column).max().expect("edge")
            })
            .collect::<Vec<_>>();
        let right_edge = right
            .vertices
            .iter()
            .filter(|vertex| {
                vertex.grid_column
                    == right.vertices.iter().map(|v| v.grid_column).min().expect("edge")
            })
            .collect::<Vec<_>>();

        assert_eq!(left_edge.len(), right_edge.len());
        for (left_vertex, right_vertex) in left_edge.iter().zip(right_edge.iter()) {
            assert_eq!(left_vertex.position, right_vertex.position);
            assert_eq!(left_vertex.normal, right_vertex.normal);
            assert_eq!(left_vertex.tangent, right_vertex.tangent);
        }
    }

    proptest! {
        #[test]
        fn mesh_stats_are_deterministic(seed in any::<u64>()) {
            let root =
                RootSeed::parse_hex(&format!("0x{seed:016X}")).expect("seed should parse");
            let config = TerrainFieldConfig::default();
            let first = TerrainScalarFieldSet::generate(root, config).expect("field set should build");
            let second = TerrainScalarFieldSet::generate(root, config).expect("field set should build");

            let first_mesh = TerrainMeshAtlas::build(&first, TerrainMeshBuildConfig::default()).expect("mesh should build");
            let second_mesh = TerrainMeshAtlas::build(&second, TerrainMeshBuildConfig::default()).expect("mesh should build");

            prop_assert_eq!(first_mesh.report(), second_mesh.report());
        }
    }

    #[test]
    fn invalid_chunk_resolution_is_rejected() {
        let error = TerrainMeshAtlas::build(
            &canonical_fields(),
            TerrainMeshBuildConfig::new(canonical_fields().cache_resolution()),
        )
        .expect_err("oversized chunks should fail");

        assert!(error.contains("must be smaller than terrain resolution"));
    }
}
