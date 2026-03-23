use serde::{Deserialize, Serialize};
use wr_math::{inverse_lerp, stable_sin_radians};
use wr_world_gen::{RedwoodForestGraphSet, RedwoodTreeGraph};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedwoodMeshLodTier {
    Hero,
    Mid,
    Far,
}

impl RedwoodMeshLodTier {
    pub const ALL: [Self; 3] = [Self::Hero, Self::Mid, Self::Far];

    pub const fn as_index(self) -> usize {
        match self {
            Self::Hero => 0,
            Self::Mid => 1,
            Self::Far => 2,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hero => "hero",
            Self::Mid => "mid",
            Self::Far => "far",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodMeshAabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl RedwoodMeshAabb {
    fn empty() -> Self {
        Self {
            min: [f32::INFINITY, f32::INFINITY, f32::INFINITY],
            max: [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
        }
    }

    fn include(&mut self, position: [f32; 3]) {
        self.min[0] = self.min[0].min(position[0]);
        self.min[1] = self.min[1].min(position[1]);
        self.min[2] = self.min[2].min(position[2]);
        self.max[0] = self.max[0].max(position[0]);
        self.max[1] = self.max[1].max(position[1]);
        self.max[2] = self.max[2].max(position[2]);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodMeshBuildConfig {
    pub radial_segments_per_lod: [u8; 3],
    pub minimum_major_branch_radius_m: f32,
    pub bark_ridge_amplitude: f32,
    pub bark_ridge_frequency: f32,
    pub bark_twist_radians_per_meter: f32,
    pub uv_repeat_per_meter: f32,
    pub cap_extension_scale: f32,
}

impl RedwoodMeshBuildConfig {
    pub fn validate(self) -> Result<Self, RedwoodMeshBuildError> {
        for (name, value) in [
            ("minimum_major_branch_radius_m", self.minimum_major_branch_radius_m),
            ("bark_ridge_amplitude", self.bark_ridge_amplitude),
            ("bark_ridge_frequency", self.bark_ridge_frequency),
            ("bark_twist_radians_per_meter", self.bark_twist_radians_per_meter.abs()),
            ("uv_repeat_per_meter", self.uv_repeat_per_meter),
            ("cap_extension_scale", self.cap_extension_scale),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(RedwoodMeshBuildError::invalid_config(format!(
                    "{name} must be finite and non-negative"
                )));
            }
        }

        for (index, segments) in self.radial_segments_per_lod.into_iter().enumerate() {
            if segments < 3 {
                return Err(RedwoodMeshBuildError::invalid_config(format!(
                    "radial_segments_per_lod[{index}] must be at least 3"
                )));
            }
        }

        if self.radial_segments_per_lod[0] < self.radial_segments_per_lod[1]
            || self.radial_segments_per_lod[1] < self.radial_segments_per_lod[2]
        {
            return Err(RedwoodMeshBuildError::invalid_config(
                "radial_segments_per_lod must monotonically decrease from hero to far",
            ));
        }

        Ok(self)
    }
}

impl Default for RedwoodMeshBuildConfig {
    fn default() -> Self {
        Self {
            radial_segments_per_lod: [12, 8, 5],
            minimum_major_branch_radius_m: 0.2,
            bark_ridge_amplitude: 0.075,
            bark_ridge_frequency: 6.0,
            bark_twist_radians_per_meter: 0.11,
            uv_repeat_per_meter: 0.18,
            cap_extension_scale: 0.18,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodMeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub uv: [f32; 2],
    pub material_params: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodMeshTriangle {
    pub tree_index: usize,
    pub lod: RedwoodMeshLodTier,
    pub indices: [u32; 3],
    pub positions: [[f32; 3]; 3],
    pub normals: [[f32; 3]; 3],
    pub tangents: [[f32; 3]; 3],
    pub uvs: [[f32; 2]; 3],
    pub material_params: [[f32; 2]; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodMeshLodReport {
    pub lod: RedwoodMeshLodTier,
    pub radial_segments: u8,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub bounds: RedwoodMeshAabb,
    pub max_radius_m: f32,
    pub mean_radius_m: f32,
    pub silhouette_extents_m: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedwoodMeshLod {
    pub lod: RedwoodMeshLodTier,
    pub radial_segments: u8,
    pub vertices: Vec<RedwoodMeshVertex>,
    pub indices: Vec<u32>,
    pub report: RedwoodMeshLodReport,
}

impl RedwoodMeshLod {
    pub fn triangles(&self, tree_index: usize) -> Vec<RedwoodMeshTriangle> {
        self.indices
            .chunks_exact(3)
            .map(|indices| {
                let a = self.vertices[indices[0] as usize];
                let b = self.vertices[indices[1] as usize];
                let c = self.vertices[indices[2] as usize];

                RedwoodMeshTriangle {
                    tree_index,
                    lod: self.lod,
                    indices: [indices[0], indices[1], indices[2]],
                    positions: [a.position, b.position, c.position],
                    normals: [a.normal, b.normal, c.normal],
                    tangents: [a.tangent, b.tangent, c.tangent],
                    uvs: [a.uv, b.uv, c.uv],
                    material_params: [a.material_params, b.material_params, c.material_params],
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodTreeMeshReport {
    pub tree_index: usize,
    pub included_node_count: usize,
    pub capped_tip_count: usize,
    pub lods: [RedwoodMeshLodReport; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedwoodTreeMesh {
    pub tree_index: usize,
    pub lods: [RedwoodMeshLod; 3],
    pub report: RedwoodTreeMeshReport,
}

impl RedwoodTreeMesh {
    pub fn lod(&self, lod: RedwoodMeshLodTier) -> &RedwoodMeshLod {
        &self.lods[lod.as_index()]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodForestMeshReport {
    pub seed_hex: String,
    pub tree_count: usize,
    pub total_included_nodes: usize,
    pub total_capped_tips: usize,
    pub lods: [RedwoodMeshLodReport; 3],
    pub trees: Vec<RedwoodTreeMeshReport>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedwoodForestMeshSet {
    seed_hex: String,
    config: RedwoodMeshBuildConfig,
    trees: Vec<RedwoodTreeMesh>,
    report: RedwoodForestMeshReport,
}

impl RedwoodForestMeshSet {
    pub fn build(
        graphs: &RedwoodForestGraphSet,
        config: RedwoodMeshBuildConfig,
    ) -> Result<Self, RedwoodMeshBuildError> {
        let config = config.validate()?;
        let mut trees = Vec::with_capacity(graphs.trees().len());

        for tree in graphs.trees() {
            trees.push(build_tree_mesh(tree, config)?);
        }

        let report = RedwoodForestMeshReport::from_trees(graphs.seed_hex().to_owned(), &trees);

        Ok(Self { seed_hex: graphs.seed_hex().to_owned(), config, trees, report })
    }

    pub fn seed_hex(&self) -> &str {
        &self.seed_hex
    }

    pub fn config(&self) -> RedwoodMeshBuildConfig {
        self.config
    }

    pub fn trees(&self) -> &[RedwoodTreeMesh] {
        &self.trees
    }

    pub fn report(&self) -> RedwoodForestMeshReport {
        self.report.clone()
    }

    pub fn triangles_for_lod(&self, lod: RedwoodMeshLodTier) -> Vec<RedwoodMeshTriangle> {
        self.trees.iter().flat_map(|tree| tree.lod(lod).triangles(tree.tree_index)).collect()
    }
}

impl RedwoodForestMeshReport {
    fn from_trees(seed_hex: String, trees: &[RedwoodTreeMesh]) -> Self {
        let mut aggregate_lods = RedwoodMeshLodAggregate::default();
        let mut total_included_nodes = 0;
        let mut total_capped_tips = 0;

        for tree in trees {
            total_included_nodes += tree.report.included_node_count;
            total_capped_tips += tree.report.capped_tip_count;
            for report in tree.report.lods {
                aggregate_lods.include(report);
            }
        }

        Self {
            seed_hex,
            tree_count: trees.len(),
            total_included_nodes,
            total_capped_tips,
            lods: aggregate_lods.finish(),
            trees: trees.iter().take(32).map(|tree| tree.report).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedwoodMeshBuildError {
    message: String,
}

impl RedwoodMeshBuildError {
    fn invalid_config(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl std::fmt::Display for RedwoodMeshBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RedwoodMeshBuildError {}

#[derive(Debug, Clone, Copy)]
struct NodeRecord {
    original_index: usize,
    parent: Option<usize>,
    position: Vec3,
    radius_m: f32,
    axis: Vec3,
    basis_u: Vec3,
    basis_v: Vec3,
    path_distance_m: f32,
    depth: u16,
}

#[derive(Debug, Clone)]
struct LodBuffers {
    lod: RedwoodMeshLodTier,
    radial_segments: u8,
    vertices: Vec<RedwoodMeshVertex>,
    indices: Vec<u32>,
    bounds: RedwoodMeshAabb,
    radius_sum: f32,
    radius_count: u32,
    max_radius_m: f32,
}

#[derive(Debug, Default)]
struct RedwoodMeshLodAggregate {
    accumulators: [LodAccumulator; 3],
}

#[derive(Debug, Clone, Copy)]
struct LodAccumulator {
    bounds: RedwoodMeshAabb,
    vertex_count: u32,
    triangle_count: u32,
    radius_sum: f32,
    radius_count: u32,
    max_radius_m: f32,
    initialized: bool,
    radial_segments: u8,
}

impl Default for LodAccumulator {
    fn default() -> Self {
        Self {
            bounds: RedwoodMeshAabb::empty(),
            vertex_count: 0,
            triangle_count: 0,
            radius_sum: 0.0,
            radius_count: 0,
            max_radius_m: 0.0,
            initialized: false,
            radial_segments: 0,
        }
    }
}

impl RedwoodMeshLodAggregate {
    fn include(&mut self, report: RedwoodMeshLodReport) {
        let accumulator = &mut self.accumulators[report.lod.as_index()];
        accumulator.radial_segments = report.radial_segments;
        accumulator.vertex_count += report.vertex_count;
        accumulator.triangle_count += report.triangle_count;
        accumulator.radius_sum += report.mean_radius_m * report.vertex_count as f32;
        accumulator.radius_count += report.vertex_count;
        accumulator.max_radius_m = accumulator.max_radius_m.max(report.max_radius_m);
        if !accumulator.initialized {
            accumulator.bounds = report.bounds;
            accumulator.initialized = true;
        } else {
            accumulator.bounds.include(report.bounds.min);
            accumulator.bounds.include(report.bounds.max);
        }
    }

    fn finish(self) -> [RedwoodMeshLodReport; 3] {
        RedwoodMeshLodTier::ALL.map(|lod| {
            let accumulator = self.accumulators[lod.as_index()];
            let mean_radius_m = if accumulator.radius_count == 0 {
                0.0
            } else {
                accumulator.radius_sum / accumulator.radius_count as f32
            };
            RedwoodMeshLodReport {
                lod,
                radial_segments: accumulator.radial_segments,
                vertex_count: accumulator.vertex_count,
                triangle_count: accumulator.triangle_count,
                bounds: accumulator.bounds,
                max_radius_m: accumulator.max_radius_m,
                mean_radius_m,
                silhouette_extents_m: [
                    accumulator.bounds.max[0] - accumulator.bounds.min[0],
                    accumulator.bounds.max[2] - accumulator.bounds.min[2],
                ],
            }
        })
    }
}

impl LodBuffers {
    fn new(lod: RedwoodMeshLodTier, radial_segments: u8) -> Self {
        Self {
            lod,
            radial_segments,
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds: RedwoodMeshAabb::empty(),
            radius_sum: 0.0,
            radius_count: 0,
            max_radius_m: 0.0,
        }
    }

    fn push_vertex(&mut self, vertex: RedwoodMeshVertex, radius_m: f32) -> u32 {
        self.bounds.include(vertex.position);
        self.radius_sum += radius_m;
        self.radius_count += 1;
        self.max_radius_m = self.max_radius_m.max(radius_m);
        self.vertices.push(vertex);
        (self.vertices.len() - 1) as u32
    }

    fn report(&self) -> RedwoodMeshLodReport {
        RedwoodMeshLodReport {
            lod: self.lod,
            radial_segments: self.radial_segments,
            vertex_count: self.vertices.len() as u32,
            triangle_count: (self.indices.len() / 3) as u32,
            bounds: self.bounds,
            max_radius_m: self.max_radius_m,
            mean_radius_m: if self.radius_count == 0 {
                0.0
            } else {
                self.radius_sum / self.radius_count as f32
            },
            silhouette_extents_m: [
                self.bounds.max[0] - self.bounds.min[0],
                self.bounds.max[2] - self.bounds.min[2],
            ],
        }
    }

    fn into_lod(self) -> RedwoodMeshLod {
        let report = self.report();
        RedwoodMeshLod {
            lod: self.lod,
            radial_segments: self.radial_segments,
            vertices: self.vertices,
            indices: self.indices,
            report,
        }
    }
}

fn build_tree_mesh(
    tree: &RedwoodTreeGraph,
    config: RedwoodMeshBuildConfig,
) -> Result<RedwoodTreeMesh, RedwoodMeshBuildError> {
    let nodes = select_mesh_nodes(tree, config.minimum_major_branch_radius_m);
    if nodes.len() < 2 {
        return Err(RedwoodMeshBuildError::invalid_config(format!(
            "tree {} did not produce enough major nodes for meshing",
            tree.tree_index
        )));
    }

    let child_counts = child_counts(&nodes);
    let capped_tip_count = child_counts.iter().filter(|count| **count == 0).count();
    let per_node_taper = (tree.max_radius_m().max(0.0001), config);
    let mut lods = RedwoodMeshLodTier::ALL
        .map(|lod| LodBuffers::new(lod, config.radial_segments_per_lod[lod.as_index()]));

    for (lod_index, lod) in lods.iter_mut().enumerate() {
        let ring_count = usize::from(config.radial_segments_per_lod[lod_index]);
        let mut node_rings = vec![Vec::<u32>::new(); nodes.len()];

        for (node_index, node) in nodes.iter().enumerate() {
            node_rings[node_index] = build_ring(
                lod,
                tree.tree_index,
                node,
                ring_count,
                per_node_taper.0,
                per_node_taper.1,
            );
        }

        for (node_index, node) in nodes.iter().enumerate() {
            if let Some(parent_index) = node.parent {
                stitch_rings(lod, &node_rings[parent_index], &node_rings[node_index]);
            } else {
                cap_ring(
                    lod,
                    &node_rings[node_index],
                    node.position - node.axis * (node.radius_m * config.cap_extension_scale),
                    -node.axis,
                    node.radius_m,
                    tree.max_radius_m().max(0.0001),
                    true,
                );
            }
        }

        for (node_index, node) in nodes.iter().enumerate() {
            if child_counts[node_index] == 0 {
                cap_ring(
                    lod,
                    &node_rings[node_index],
                    node.position + node.axis * (node.radius_m * config.cap_extension_scale),
                    node.axis,
                    node.radius_m,
                    tree.max_radius_m().max(0.0001),
                    false,
                );
            }
        }
    }

    let lods = lods.map(LodBuffers::into_lod);
    let report = RedwoodTreeMeshReport {
        tree_index: tree.tree_index,
        included_node_count: nodes.len(),
        capped_tip_count,
        lods: [lods[0].report, lods[1].report, lods[2].report],
    };

    Ok(RedwoodTreeMesh { tree_index: tree.tree_index, lods, report })
}

fn select_mesh_nodes(tree: &RedwoodTreeGraph, min_radius_m: f32) -> Vec<NodeRecord> {
    let mut include = vec![false; tree.nodes.len()];
    include[0] = true;
    for (index, node) in tree.nodes.iter().enumerate().skip(1) {
        if node.radius_m >= min_radius_m
            || tree.nodes[node.parent_index.expect("parent")].radius_m >= min_radius_m
        {
            include[index] = true;
        }
    }

    let mut remap = vec![None; tree.nodes.len()];
    let mut nodes: Vec<NodeRecord> = Vec::new();

    for (original_index, node) in tree.nodes.iter().enumerate() {
        if !include[original_index] {
            continue;
        }

        let parent: Option<usize> = node.parent_index.and_then(|parent_index| remap[parent_index]);
        let path_distance_m = if let Some(parent_index) = parent {
            let parent_record = &nodes[parent_index];
            parent_record.path_distance_m
                + distance3(
                    point3_to_vec3(tree.nodes[parent_record.original_index].position),
                    point3_to_vec3(node.position),
                )
        } else {
            0.0
        };

        nodes.push(NodeRecord {
            original_index,
            parent,
            position: point3_to_vec3(node.position),
            radius_m: node.radius_m,
            axis: Vec3::ZERO,
            basis_u: Vec3::ZERO,
            basis_v: Vec3::ZERO,
            path_distance_m,
            depth: node.depth,
        });
        remap[original_index] = Some(nodes.len() - 1);
    }

    for index in 0..nodes.len() {
        let original_index = nodes[index].original_index;
        let parent_direction = nodes[index]
            .parent
            .map(|parent_index| (nodes[index].position - nodes[parent_index].position).normalize());
        let child_directions: Vec<Vec3> = tree.nodes[original_index]
            .children
            .iter()
            .filter_map(|child_index| remap[*child_index])
            .map(|child_index| (nodes[child_index].position - nodes[index].position).normalize())
            .collect::<Vec<_>>();

        let axis = match (parent_direction, child_directions.as_slice()) {
            (Some(parent), children) if !children.is_empty() => {
                let child_average = children.iter().copied().fold(Vec3::ZERO, |sum, dir| sum + dir)
                    / children.len() as f32;
                (parent + child_average).normalize()
            }
            (Some(parent), []) => parent,
            (None, children) if !children.is_empty() => {
                children.iter().copied().fold(Vec3::ZERO, |sum, dir| sum + dir).normalize()
            }
            _ => Vec3::Z,
        };
        let basis_u = axis.perpendicular();
        let basis_v = axis.cross(basis_u).normalize();

        nodes[index].axis = axis;
        nodes[index].basis_u = basis_u;
        nodes[index].basis_v = basis_v;
    }

    nodes
}

fn child_counts(nodes: &[NodeRecord]) -> Vec<usize> {
    let mut counts = vec![0; nodes.len()];
    for node in nodes {
        if let Some(parent_index) = node.parent {
            counts[parent_index] += 1;
        }
    }
    counts
}

fn build_ring(
    lod: &mut LodBuffers,
    tree_index: usize,
    node: &NodeRecord,
    ring_count: usize,
    max_tree_radius_m: f32,
    config: RedwoodMeshBuildConfig,
) -> Vec<u32> {
    let mut ring = Vec::with_capacity(ring_count);
    let taper_t = 1.0 - inverse_lerp(0.0, max_tree_radius_m, node.radius_m).clamp(0.0, 1.0);

    for segment in 0..ring_count {
        let angle = (segment as f32 / ring_count as f32) * std::f32::consts::TAU;
        let bark_wave = bark_ridge(tree_index, node, angle, config);
        let radius = node.radius_m * (1.0 + (config.bark_ridge_amplitude * bark_wave));
        let outward = ((node.basis_u * angle.cos()) + (node.basis_v * angle.sin())).normalize();
        let tangent = node.axis;
        let position = node.position + outward * radius;
        let vertex = RedwoodMeshVertex {
            position: position.into_array(),
            normal: outward.into_array(),
            tangent: tangent.into_array(),
            uv: [
                segment as f32 / ring_count as f32,
                node.path_distance_m * config.uv_repeat_per_meter,
            ],
            material_params: [taper_t, bark_wave.max(0.0)],
        };
        ring.push(lod.push_vertex(vertex, radius));
    }

    ring
}

fn bark_ridge(
    tree_index: usize,
    node: &NodeRecord,
    angle: f32,
    config: RedwoodMeshBuildConfig,
) -> f32 {
    let phase = angle * config.bark_ridge_frequency
        + (node.path_distance_m * config.bark_twist_radians_per_meter)
        + ((tree_index as f32) * 0.37)
        + ((node.depth as f32) * 0.19);
    stable_sin_radians(phase)
}

fn stitch_rings(lod: &mut LodBuffers, parent_ring: &[u32], child_ring: &[u32]) {
    for segment in 0..parent_ring.len() {
        let next = (segment + 1) % parent_ring.len();
        lod.indices.extend_from_slice(&[
            parent_ring[segment],
            child_ring[segment],
            parent_ring[next],
            parent_ring[next],
            child_ring[segment],
            child_ring[next],
        ]);
    }
}

fn cap_ring(
    lod: &mut LodBuffers,
    ring: &[u32],
    center: Vec3,
    normal: Vec3,
    radius_m: f32,
    max_tree_radius_m: f32,
    reverse: bool,
) {
    let center_index = lod.push_vertex(
        RedwoodMeshVertex {
            position: center.into_array(),
            normal: normal.into_array(),
            tangent: normal.perpendicular().into_array(),
            uv: [0.5, 0.0],
            material_params: [
                1.0 - inverse_lerp(0.0, max_tree_radius_m, radius_m).clamp(0.0, 1.0),
                0.0,
            ],
        },
        0.0,
    );

    for segment in 0..ring.len() {
        let next = (segment + 1) % ring.len();
        if reverse {
            lod.indices.extend_from_slice(&[center_index, ring[next], ring[segment]]);
        } else {
            lod.indices.extend_from_slice(&[center_index, ring[segment], ring[next]]);
        }
    }
}

fn point3_to_vec3(point: wr_world_gen::RedwoodPoint3) -> Vec3 {
    Vec3::new(point.x_m, point.y_m, point.z_m)
}

fn distance3(a: Vec3, b: Vec3) -> f32 {
    (b - a).length()
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    const Z: Self = Self::new(0.0, 0.0, 1.0);

    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    const fn into_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    fn normalize(self) -> Self {
        let length = self.length();
        if length <= f32::EPSILON { Self::Z } else { self / length }
    }

    fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    fn perpendicular(self) -> Self {
        let fallback = if self.z.abs() < 0.95 { Self::Z } else { Self::new(1.0, 0.0, 0.0) };
        self.cross(fallback).normalize()
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl std::ops::Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_json_snapshot;
    use proptest::prelude::*;
    use wr_world_gen::{
        EcologicalPlacementConfig, EcologicalPlacementSet, RedwoodForestGraphConfig,
        RedwoodForestGraphSet, TerrainFieldConfig, TerrainScalarFieldSet,
    };
    use wr_world_seed::RootSeed;

    use super::*;

    fn canonical_graphs(seed_hex: &str) -> RedwoodForestGraphSet {
        let seed = RootSeed::parse_hex(seed_hex).expect("seed should parse");
        let fields = TerrainScalarFieldSet::generate(seed, TerrainFieldConfig::default())
            .expect("fields should generate");
        let placements =
            EcologicalPlacementSet::generate(seed, &fields, EcologicalPlacementConfig::default())
                .expect("placements should generate");
        RedwoodForestGraphSet::generate(
            seed,
            &fields,
            &placements,
            RedwoodForestGraphConfig::default(),
        )
        .expect("graphs should generate")
    }

    fn compact_report_snapshot(report: &RedwoodForestMeshReport) -> serde_json::Value {
        serde_json::json!({
            "seed_hex": report.seed_hex,
            "tree_count": report.tree_count,
            "total_included_nodes": report.total_included_nodes,
            "total_capped_tips": report.total_capped_tips,
            "lods": report.lods.iter().map(|lod| {
                serde_json::json!({
                    "lod": lod.lod.as_str(),
                    "vertex_count": lod.vertex_count,
                    "triangle_count": lod.triangle_count,
                    "max_radius_m": format!("{:.3}", lod.max_radius_m),
                    "mean_radius_m": format!("{:.3}", lod.mean_radius_m),
                    "silhouette_extents_m": [
                        format!("{:.3}", lod.silhouette_extents_m[0]),
                        format!("{:.3}", lod.silhouette_extents_m[1]),
                    ],
                })
            }).collect::<Vec<_>>(),
            "trees": report.trees.iter().take(3).map(|tree| {
                serde_json::json!({
                    "tree_index": tree.tree_index,
                    "included_node_count": tree.included_node_count,
                    "capped_tip_count": tree.capped_tip_count,
                    "lods": tree.lods.iter().map(|lod| serde_json::json!({
                        "lod": lod.lod.as_str(),
                        "vertex_count": lod.vertex_count,
                        "triangle_count": lod.triangle_count,
                    })).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
        })
    }

    #[test]
    fn forest_mesh_report_matches_snapshot() {
        let graphs = canonical_graphs("0xDEADBEEF");
        let mesh = RedwoodForestMeshSet::build(&graphs, RedwoodMeshBuildConfig::default())
            .expect("mesh set should build");

        assert_json_snapshot!(compact_report_snapshot(&mesh.report()), @r#"
        {
          "lods": [
            {
              "lod": "hero",
              "max_radius_m": "2.541",
              "mean_radius_m": "0.549",
              "silhouette_extents_m": [
                "523.147",
                "116.890"
              ],
              "triangle_count": 192228,
              "vertex_count": 92419
            },
            {
              "lod": "mid",
              "max_radius_m": "2.629",
              "mean_radius_m": "0.545",
              "silhouette_extents_m": [
                "523.147",
                "116.890"
              ],
              "triangle_count": 128152,
              "vertex_count": 62079
            },
            {
              "lod": "far",
              "max_radius_m": "2.608",
              "mean_radius_m": "0.537",
              "silhouette_extents_m": [
                "523.135",
                "116.880"
              ],
              "triangle_count": 80095,
              "vertex_count": 39324
            }
          ],
          "seed_hex": "0x00000000DEADBEEF",
          "total_capped_tips": 1124,
          "total_included_nodes": 7585,
          "tree_count": 275,
          "trees": [
            {
              "capped_tip_count": 4,
              "included_node_count": 28,
              "lods": [
                {
                  "lod": "hero",
                  "triangle_count": 708,
                  "vertex_count": 341
                },
                {
                  "lod": "mid",
                  "triangle_count": 472,
                  "vertex_count": 229
                },
                {
                  "lod": "far",
                  "triangle_count": 295,
                  "vertex_count": 145
                }
              ],
              "tree_index": 0
            },
            {
              "capped_tip_count": 4,
              "included_node_count": 28,
              "lods": [
                {
                  "lod": "hero",
                  "triangle_count": 708,
                  "vertex_count": 341
                },
                {
                  "lod": "mid",
                  "triangle_count": 472,
                  "vertex_count": 229
                },
                {
                  "lod": "far",
                  "triangle_count": 295,
                  "vertex_count": 145
                }
              ],
              "tree_index": 1
            },
            {
              "capped_tip_count": 4,
              "included_node_count": 28,
              "lods": [
                {
                  "lod": "hero",
                  "triangle_count": 708,
                  "vertex_count": 341
                },
                {
                  "lod": "mid",
                  "triangle_count": 472,
                  "vertex_count": 229
                },
                {
                  "lod": "far",
                  "triangle_count": 295,
                  "vertex_count": 145
                }
              ],
              "tree_index": 2
            }
          ]
        }
        "#);
    }

    #[test]
    fn lod_triangle_counts_monotonically_decrease_without_changing_bounds() {
        let graphs = canonical_graphs("0xC0FFEE01");
        let mesh = RedwoodForestMeshSet::build(&graphs, RedwoodMeshBuildConfig::default())
            .expect("mesh set should build");

        for tree in mesh.trees() {
            let hero = tree.lod(RedwoodMeshLodTier::Hero).report;
            let mid = tree.lod(RedwoodMeshLodTier::Mid).report;
            let far = tree.lod(RedwoodMeshLodTier::Far).report;

            assert!(hero.triangle_count > mid.triangle_count);
            assert!(mid.triangle_count > far.triangle_count);
            for axis in 0..3 {
                assert!(
                    (hero.bounds.min[axis] - mid.bounds.min[axis]).abs() < 0.75,
                    "hero->mid min bound drifted too far on axis {axis}"
                );
                assert!(
                    (hero.bounds.max[axis] - mid.bounds.max[axis]).abs() < 0.75,
                    "hero->mid max bound drifted too far on axis {axis}"
                );
                assert!(
                    (mid.bounds.min[axis] - far.bounds.min[axis]).abs() < 0.75,
                    "mid->far min bound drifted too far on axis {axis}"
                );
                assert!(
                    (mid.bounds.max[axis] - far.bounds.max[axis]).abs() < 0.75,
                    "mid->far max bound drifted too far on axis {axis}"
                );
            }
        }
    }

    #[test]
    fn invalid_config_is_rejected() {
        let error = RedwoodMeshBuildConfig {
            radial_segments_per_lod: [4, 6, 3],
            ..RedwoodMeshBuildConfig::default()
        }
        .validate()
        .expect_err("lod segment counts should be monotonic");

        assert!(error.to_string().contains("monotonically decrease"));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(8))]

        #[test]
        fn generated_meshes_stay_finite_and_non_degenerate(seed in any::<u64>()) {
            let seed = RootSeed::parse_hex(&format!("0x{seed:016X}")).expect("seed should parse");
            let fields = TerrainScalarFieldSet::generate(seed, TerrainFieldConfig::default()).expect("fields should generate");
            let placements = EcologicalPlacementSet::generate(seed, &fields, EcologicalPlacementConfig::default()).expect("placements should generate");
            let graphs = RedwoodForestGraphSet::generate(seed, &fields, &placements, RedwoodForestGraphConfig::default()).expect("graphs should generate");
            let mesh = RedwoodForestMeshSet::build(&graphs, RedwoodMeshBuildConfig::default()).expect("mesh should build");

            for tree in mesh.trees() {
                let lod = tree.lod(RedwoodMeshLodTier::Mid);
                for triangle in lod.triangles(tree.tree_index) {
                    for position in triangle.positions {
                        prop_assert!(position.into_iter().all(f32::is_finite));
                    }

                    let ab = Vec3::new(
                        triangle.positions[1][0] - triangle.positions[0][0],
                        triangle.positions[1][1] - triangle.positions[0][1],
                        triangle.positions[1][2] - triangle.positions[0][2],
                    );
                    let ac = Vec3::new(
                        triangle.positions[2][0] - triangle.positions[0][0],
                        triangle.positions[2][1] - triangle.positions[0][1],
                        triangle.positions[2][2] - triangle.positions[0][2],
                    );
                    prop_assert!(ab.cross(ac).length() > 0.00001);
                }
            }
        }
    }
}
