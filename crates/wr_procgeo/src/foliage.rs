use serde::{Deserialize, Serialize};
use wr_math::inverse_lerp;
use wr_world_gen::{RedwoodForestGraphSet, RedwoodNode, RedwoodPoint3, RedwoodTreeGraph};

use crate::{RedwoodMeshAabb, RedwoodMeshLodTier};

const PACK_SCALE_8BIT: f32 = 255.0;
const MAX_REPORT_TREES: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodFoliageBuildConfig {
    pub cards_per_cluster_per_lod: [u8; 3],
    pub cluster_radius_m: [f32; 2],
    pub card_aspect_range: [f32; 2],
    pub radial_jitter_m: f32,
    pub tip_forward_offset_m: f32,
    pub cluster_vertical_bias_m: f32,
}

impl RedwoodFoliageBuildConfig {
    pub fn validate(self) -> Result<Self, RedwoodFoliageBuildError> {
        for (name, value) in [
            ("cluster_radius_min_m", self.cluster_radius_m[0]),
            ("cluster_radius_max_m", self.cluster_radius_m[1]),
            ("card_aspect_min", self.card_aspect_range[0]),
            ("card_aspect_max", self.card_aspect_range[1]),
            ("radial_jitter_m", self.radial_jitter_m),
            ("tip_forward_offset_m", self.tip_forward_offset_m),
            ("cluster_vertical_bias_m", self.cluster_vertical_bias_m.abs()),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(RedwoodFoliageBuildError::invalid_config(format!(
                    "{name} must be finite and non-negative"
                )));
            }
        }

        if self.cluster_radius_m[0] <= 0.0 || self.cluster_radius_m[0] > self.cluster_radius_m[1] {
            return Err(RedwoodFoliageBuildError::invalid_config(
                "cluster_radius_m must be an increasing positive range",
            ));
        }

        if self.card_aspect_range[0] <= 0.0 || self.card_aspect_range[0] > self.card_aspect_range[1]
        {
            return Err(RedwoodFoliageBuildError::invalid_config(
                "card_aspect_range must be an increasing positive range",
            ));
        }

        for (index, cards) in self.cards_per_cluster_per_lod.into_iter().enumerate() {
            if cards < 2 {
                return Err(RedwoodFoliageBuildError::invalid_config(format!(
                    "cards_per_cluster_per_lod[{index}] must be at least 2"
                )));
            }
        }

        if self.cards_per_cluster_per_lod[0] < self.cards_per_cluster_per_lod[1]
            || self.cards_per_cluster_per_lod[1] < self.cards_per_cluster_per_lod[2]
        {
            return Err(RedwoodFoliageBuildError::invalid_config(
                "cards_per_cluster_per_lod must monotonically decrease from hero to far",
            ));
        }

        Ok(self)
    }
}

impl Default for RedwoodFoliageBuildConfig {
    fn default() -> Self {
        Self {
            cards_per_cluster_per_lod: [5, 4, 3],
            cluster_radius_m: [1.4, 3.4],
            card_aspect_range: [1.35, 1.95],
            radial_jitter_m: 0.65,
            tip_forward_offset_m: 0.55,
            cluster_vertical_bias_m: 0.8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedwoodFoliageBudget {
    pub max_total_cards_per_lod: [u32; 3],
    pub max_total_vertices_per_lod: [u32; 3],
    pub max_draw_calls_per_lod: [u8; 3],
}

impl RedwoodFoliageBudget {
    pub const fn within(self, report: &RedwoodForestFoliageReport) -> bool {
        report.lods[0].card_count <= self.max_total_cards_per_lod[0]
            && report.lods[1].card_count <= self.max_total_cards_per_lod[1]
            && report.lods[2].card_count <= self.max_total_cards_per_lod[2]
            && report.lods[0].vertex_count <= self.max_total_vertices_per_lod[0]
            && report.lods[1].vertex_count <= self.max_total_vertices_per_lod[1]
            && report.lods[2].vertex_count <= self.max_total_vertices_per_lod[2]
            && report.lods[0].estimated_draw_calls <= self.max_draw_calls_per_lod[0]
            && report.lods[1].estimated_draw_calls <= self.max_draw_calls_per_lod[1]
            && report.lods[2].estimated_draw_calls <= self.max_draw_calls_per_lod[2]
    }
}

impl Default for RedwoodFoliageBudget {
    fn default() -> Self {
        Self {
            max_total_cards_per_lod: [6_200, 5_000, 4_000],
            max_total_vertices_per_lod: [24_800, 20_000, 16_000],
            max_draw_calls_per_lod: [1, 1, 1],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodFoliageMaterialParams {
    pub alpha_power: f32,
    pub edge_softness: f32,
    pub normal_bend: f32,
    pub backlight_boost: f32,
    pub hue_shift: f32,
    pub canopy_height_t: f32,
}

impl RedwoodFoliageMaterialParams {
    pub fn pack(self) -> RedwoodFoliageMaterialPacking {
        RedwoodFoliageMaterialPacking {
            words: [
                pack_u8x4([
                    self.alpha_power,
                    self.edge_softness,
                    self.normal_bend,
                    self.backlight_boost,
                ]),
                pack_u8x4([self.hue_shift * 0.5 + 0.5, self.canopy_height_t, 0.0, 0.0]),
            ],
        }
    }

    pub fn unpack(packing: RedwoodFoliageMaterialPacking) -> Self {
        let primary = unpack_u8x4(packing.words[0]);
        let secondary = unpack_u8x4(packing.words[1]);
        Self {
            alpha_power: primary[0],
            edge_softness: primary[1],
            normal_bend: primary[2],
            backlight_boost: primary[3],
            hue_shift: secondary[0] * 2.0 - 1.0,
            canopy_height_t: secondary[1],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedwoodFoliageMaterialPacking {
    pub words: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodFoliageCard {
    pub center: [f32; 3],
    pub axis_u: [f32; 3],
    pub axis_v: [f32; 3],
    pub normal: [f32; 3],
    pub half_extents_m: [f32; 2],
    pub packed_material_params: RedwoodFoliageMaterialPacking,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodFoliageCluster {
    pub anchor: [f32; 3],
    pub envelope_radius_m: f32,
    pub cards: Vec<RedwoodFoliageCard>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodFoliageLodReport {
    pub lod: RedwoodMeshLodTier,
    pub cluster_count: u32,
    pub card_count: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub estimated_draw_calls: u8,
    pub bounds: RedwoodMeshAabb,
    pub mean_cluster_radius_m: f32,
    pub max_card_half_extent_m: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodFoliageLod {
    pub lod: RedwoodMeshLodTier,
    pub clusters: Vec<RedwoodFoliageCluster>,
    pub report: RedwoodFoliageLodReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodFoliageTreeReport {
    pub tree_index: usize,
    pub tip_cluster_count: usize,
    pub lods: [RedwoodFoliageLodReport; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodFoliageTree {
    pub tree_index: usize,
    pub lods: [RedwoodFoliageLod; 3],
    pub report: RedwoodFoliageTreeReport,
}

impl RedwoodFoliageTree {
    pub fn lod(&self, lod: RedwoodMeshLodTier) -> &RedwoodFoliageLod {
        &self.lods[lod.as_index()]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodForestFoliageReport {
    pub seed_hex: String,
    pub tree_count: usize,
    pub total_tip_clusters: usize,
    pub trees_truncated: bool,
    pub budget: RedwoodFoliageBudget,
    pub within_budget: bool,
    pub lods: [RedwoodFoliageLodReport; 3],
    pub trees: Vec<RedwoodFoliageTreeReport>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedwoodForestFoliageSet {
    seed_hex: String,
    config: RedwoodFoliageBuildConfig,
    trees: Vec<RedwoodFoliageTree>,
    report: RedwoodForestFoliageReport,
}

impl RedwoodForestFoliageSet {
    pub fn build(
        graphs: &RedwoodForestGraphSet,
        config: RedwoodFoliageBuildConfig,
    ) -> Result<Self, RedwoodFoliageBuildError> {
        let config = config.validate()?;
        let mut trees = Vec::with_capacity(graphs.trees().len());

        for tree in graphs.trees() {
            trees.push(build_tree_foliage(tree, config)?);
        }

        let budget = RedwoodFoliageBudget::default();
        let report =
            RedwoodForestFoliageReport::from_trees(graphs.seed_hex().to_owned(), &trees, budget);

        Ok(Self { seed_hex: graphs.seed_hex().to_owned(), config, trees, report })
    }

    pub fn config(&self) -> RedwoodFoliageBuildConfig {
        self.config
    }

    pub fn seed_hex(&self) -> &str {
        &self.seed_hex
    }

    pub fn trees(&self) -> &[RedwoodFoliageTree] {
        &self.trees
    }

    pub fn report(&self) -> RedwoodForestFoliageReport {
        self.report.clone()
    }
}

impl RedwoodForestFoliageReport {
    fn from_trees(
        seed_hex: String,
        trees: &[RedwoodFoliageTree],
        budget: RedwoodFoliageBudget,
    ) -> Self {
        let mut aggregates = RedwoodMeshLodTier::ALL.map(FoliageAggregate::new);
        let mut total_tip_clusters = 0;

        for tree in trees {
            total_tip_clusters += tree.report.tip_cluster_count;
            for (index, lod_report) in tree.report.lods.iter().copied().enumerate() {
                aggregates[index].accumulate(lod_report);
            }
        }

        let lods = [aggregates[0].finish(), aggregates[1].finish(), aggregates[2].finish()];

        let provisional = Self {
            seed_hex,
            tree_count: trees.len(),
            total_tip_clusters,
            trees_truncated: trees.len() > MAX_REPORT_TREES,
            budget,
            within_budget: false,
            lods,
            trees: trees.iter().take(MAX_REPORT_TREES).map(|tree| tree.report).collect(),
        };

        Self { within_budget: budget.within(&provisional), ..provisional }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedwoodFoliageBuildError {
    InvalidConfig(String),
}

impl RedwoodFoliageBuildError {
    fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig(message.into())
    }
}

impl std::fmt::Display for RedwoodFoliageBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(f, "invalid foliage build config: {message}"),
        }
    }
}

impl std::error::Error for RedwoodFoliageBuildError {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct NodeClusterInput {
    anchor: Vec3,
    axis: Vec3,
    cluster_radius_m: f32,
    canopy_height_t: f32,
    tip_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FoliageAggregate {
    lod: RedwoodMeshLodTier,
    cluster_count: u32,
    card_count: u32,
    vertex_count: u32,
    triangle_count: u32,
    bounds: RedwoodMeshAabb,
    radius_sum: f32,
    radius_count: u32,
    max_card_half_extent_m: [f32; 2],
    estimated_draw_calls: u8,
}

impl FoliageAggregate {
    fn new(lod: RedwoodMeshLodTier) -> Self {
        Self {
            lod,
            cluster_count: 0,
            card_count: 0,
            vertex_count: 0,
            triangle_count: 0,
            bounds: RedwoodMeshAabb {
                min: [f32::INFINITY, f32::INFINITY, f32::INFINITY],
                max: [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
            },
            radius_sum: 0.0,
            radius_count: 0,
            max_card_half_extent_m: [0.0, 0.0],
            estimated_draw_calls: 0,
        }
    }

    fn accumulate(&mut self, report: RedwoodFoliageLodReport) {
        self.cluster_count += report.cluster_count;
        self.card_count += report.card_count;
        self.vertex_count += report.vertex_count;
        self.triangle_count += report.triangle_count;
        self.estimated_draw_calls = self.estimated_draw_calls.max(report.estimated_draw_calls);
        self.max_card_half_extent_m[0] =
            self.max_card_half_extent_m[0].max(report.max_card_half_extent_m[0]);
        self.max_card_half_extent_m[1] =
            self.max_card_half_extent_m[1].max(report.max_card_half_extent_m[1]);
        if report.cluster_count > 0 {
            self.radius_sum += report.mean_cluster_radius_m * report.cluster_count as f32;
            self.radius_count += report.cluster_count;
        }
        include_bounds(&mut self.bounds, report.bounds.min);
        include_bounds(&mut self.bounds, report.bounds.max);
    }

    fn finish(self) -> RedwoodFoliageLodReport {
        RedwoodFoliageLodReport {
            lod: self.lod,
            cluster_count: self.cluster_count,
            card_count: self.card_count,
            vertex_count: self.vertex_count,
            triangle_count: self.triangle_count,
            estimated_draw_calls: self.estimated_draw_calls.max(u8::from(self.card_count > 0)),
            bounds: self.bounds,
            mean_cluster_radius_m: if self.radius_count == 0 {
                0.0
            } else {
                self.radius_sum / self.radius_count as f32
            },
            max_card_half_extent_m: self.max_card_half_extent_m,
        }
    }
}

fn build_tree_foliage(
    tree: &RedwoodTreeGraph,
    config: RedwoodFoliageBuildConfig,
) -> Result<RedwoodFoliageTree, RedwoodFoliageBuildError> {
    let cluster_inputs = cluster_inputs(tree, config);
    let lods = RedwoodMeshLodTier::ALL.map(|lod| {
        build_lod_clusters(tree.tree_index, tree.root_position, &cluster_inputs, config, lod)
    });

    let report = RedwoodFoliageTreeReport {
        tree_index: tree.tree_index,
        tip_cluster_count: cluster_inputs.len(),
        lods: [lods[0].report, lods[1].report, lods[2].report],
    };

    Ok(RedwoodFoliageTree { tree_index: tree.tree_index, lods, report })
}

fn cluster_inputs(
    tree: &RedwoodTreeGraph,
    config: RedwoodFoliageBuildConfig,
) -> Vec<NodeClusterInput> {
    let canopy_base_z = tree.root_position.z_m + tree.total_height_m * 0.36;
    let canopy_top_z = tree.root_position.z_m + tree.total_height_m;
    let mut inputs = Vec::new();

    for node in tree.nodes.iter().filter(|node| node.children.is_empty()) {
        let axis = tip_axis(tree, node);
        let canopy_height_t =
            inverse_lerp(canopy_base_z, canopy_top_z, node.position.z_m).clamp(0.0, 1.0);
        let envelope_radius =
            lerp(config.cluster_radius_m[0], config.cluster_radius_m[1], canopy_height_t);
        let jitter_phase = hash01(tree.tree_index, node.id, 3);
        let radial_jitter = (jitter_phase * 2.0 - 1.0) * config.radial_jitter_m;
        let lateral = axis.perpendicular() * radial_jitter;
        let anchor = point3_to_vec3(node.position)
            + axis * (config.tip_forward_offset_m + node.radius_m * 0.4)
            + lateral
            + Vec3::new(0.0, 0.0, config.cluster_vertical_bias_m * (0.3 + canopy_height_t * 0.7));
        inputs.push(NodeClusterInput {
            anchor,
            axis,
            cluster_radius_m: envelope_radius,
            canopy_height_t,
            tip_index: node.id,
        });
    }

    inputs
}

fn build_lod_clusters(
    tree_index: usize,
    root_position: RedwoodPoint3,
    inputs: &[NodeClusterInput],
    config: RedwoodFoliageBuildConfig,
    lod: RedwoodMeshLodTier,
) -> RedwoodFoliageLod {
    let cards_per_cluster = usize::from(config.cards_per_cluster_per_lod[lod.as_index()]);
    let mut clusters = Vec::with_capacity(inputs.len());
    let mut bounds = RedwoodMeshAabb {
        min: [f32::INFINITY, f32::INFINITY, f32::INFINITY],
        max: [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
    };
    let mut radius_sum = 0.0;
    let mut max_half_extents = [0.0_f32, 0.0_f32];

    for input in inputs {
        let cluster =
            build_cluster(tree_index, root_position, *input, config, lod, cards_per_cluster);
        radius_sum += input.cluster_radius_m;
        for card in &cluster.cards {
            for corner in card_corners(*card) {
                include_bounds(&mut bounds, corner.into_array());
            }
            max_half_extents[0] = max_half_extents[0].max(card.half_extents_m[0]);
            max_half_extents[1] = max_half_extents[1].max(card.half_extents_m[1]);
        }
        clusters.push(cluster);
    }

    let cluster_count = clusters.len() as u32;
    let card_count = clusters.iter().map(|cluster| cluster.cards.len()).sum::<usize>() as u32;
    let report = RedwoodFoliageLodReport {
        lod,
        cluster_count,
        card_count,
        vertex_count: card_count * 4,
        triangle_count: card_count * 2,
        estimated_draw_calls: u8::from(card_count > 0),
        bounds,
        mean_cluster_radius_m: if cluster_count == 0 {
            0.0
        } else {
            radius_sum / cluster_count as f32
        },
        max_card_half_extent_m: max_half_extents,
    };

    RedwoodFoliageLod { lod, clusters, report }
}

fn build_cluster(
    tree_index: usize,
    root_position: RedwoodPoint3,
    input: NodeClusterInput,
    config: RedwoodFoliageBuildConfig,
    lod: RedwoodMeshLodTier,
    cards_per_cluster: usize,
) -> RedwoodFoliageCluster {
    let mut cards = Vec::with_capacity(cards_per_cluster);
    let axis_u = input.axis.perpendicular();
    let axis_v = input.axis.cross(axis_u).normalize();

    for card_index in 0..cards_per_cluster {
        let orbit_t = card_index as f32 / cards_per_cluster as f32;
        let rotation = orbit_t * std::f32::consts::TAU
            + hash01(tree_index, input.tip_index, card_index as u32) * 0.55;
        let planar = axis_u * rotation.cos() + axis_v * rotation.sin();
        let cluster_spread = input.cluster_radius_m * lerp(0.18, 0.42, orbit_t);
        let center = input.anchor + planar * cluster_spread;
        let aspect = lerp(
            config.card_aspect_range[0],
            config.card_aspect_range[1],
            hash01(tree_index, input.tip_index, card_index as u32 + 17),
        );
        let width = input.cluster_radius_m
            * lerp(0.34, 0.58, hash01(tree_index, input.tip_index, card_index as u32 + 29));
        let height = width * aspect;
        let billboard_u = (planar * 0.84 + axis_u * 0.16).normalize();
        let billboard_v = (Vec3::Z * 0.72 + input.axis * 0.28).normalize();
        let normal = billboard_u.cross(billboard_v).normalize();
        let material = RedwoodFoliageMaterialParams {
            alpha_power: lerp(0.46, 0.82, input.canopy_height_t),
            edge_softness: lerp(0.18, 0.42, orbit_t),
            normal_bend: lerp(
                0.38,
                0.72,
                hash01(tree_index, input.tip_index, card_index as u32 + 43),
            ),
            backlight_boost: lerp(0.24, 0.68, input.canopy_height_t),
            hue_shift: lerp(
                -0.16,
                0.08,
                hash01(tree_index, input.tip_index, card_index as u32 + 61),
            ),
            canopy_height_t: input.canopy_height_t,
        };
        let lod_scale = match lod {
            RedwoodMeshLodTier::Hero => 1.0,
            RedwoodMeshLodTier::Mid => 1.08,
            RedwoodMeshLodTier::Far => 1.16,
        };
        let root_bias = inverse_lerp(
            root_position.z_m,
            root_position.z_m + 120.0,
            center.z.clamp(root_position.z_m, root_position.z_m + 120.0),
        );
        cards.push(RedwoodFoliageCard {
            center: center.into_array(),
            axis_u: billboard_u.into_array(),
            axis_v: billboard_v.into_array(),
            normal: (normal * lerp(0.82, 1.0, root_bias)).normalize().into_array(),
            half_extents_m: [width * 0.5 * lod_scale, height * 0.5 * lod_scale],
            packed_material_params: material.pack(),
        });
    }

    RedwoodFoliageCluster {
        anchor: input.anchor.into_array(),
        envelope_radius_m: input.cluster_radius_m,
        cards,
    }
}

fn tip_axis(tree: &RedwoodTreeGraph, node: &RedwoodNode) -> Vec3 {
    node.parent_index
        .map(|parent_index| {
            let parent = tree.nodes[parent_index].position;
            (point3_to_vec3(node.position) - point3_to_vec3(parent)).normalize()
        })
        .unwrap_or(Vec3::Z)
}

fn card_corners(card: RedwoodFoliageCard) -> [Vec3; 4] {
    let center = Vec3::from_array(card.center);
    let axis_u = Vec3::from_array(card.axis_u) * card.half_extents_m[0];
    let axis_v = Vec3::from_array(card.axis_v) * card.half_extents_m[1];
    [
        center - axis_u - axis_v,
        center + axis_u - axis_v,
        center + axis_u + axis_v,
        center - axis_u + axis_v,
    ]
}

fn point3_to_vec3(point: RedwoodPoint3) -> Vec3 {
    Vec3::new(point.x_m, point.y_m, point.z_m)
}

fn include_bounds(bounds: &mut RedwoodMeshAabb, position: [f32; 3]) {
    debug_assert!(
        position.iter().all(|component| component.is_finite()),
        "foliage bounds received a non-finite position: {position:?}"
    );
    if !position.iter().all(|component| component.is_finite()) {
        return;
    }
    bounds.min[0] = bounds.min[0].min(position[0]);
    bounds.min[1] = bounds.min[1].min(position[1]);
    bounds.min[2] = bounds.min[2].min(position[2]);
    bounds.max[0] = bounds.max[0].max(position[0]);
    bounds.max[1] = bounds.max[1].max(position[1]);
    bounds.max[2] = bounds.max[2].max(position[2]);
}

fn hash01(tree_index: usize, node_index: usize, salt: u32) -> f32 {
    let mut value = tree_index as u32 ^ ((node_index as u32).wrapping_mul(0x45D9F3B));
    value ^= salt.wrapping_mul(0x27D4EB2D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x2C1B3C6D);
    value ^= value >> 12;
    value = value.wrapping_mul(0x297A2D39);
    value ^= value >> 15;
    value as f32 / u32::MAX as f32
}

fn pack_u8x4(values: [f32; 4]) -> u32 {
    values.into_iter().enumerate().fold(0_u32, |packed, (index, value)| {
        let quantized = (value.clamp(0.0, 1.0) * PACK_SCALE_8BIT).round() as u32;
        packed | (quantized << (index * 8))
    })
}

fn unpack_u8x4(word: u32) -> [f32; 4] {
    [
        ((word & 0xFF) as f32) / PACK_SCALE_8BIT,
        (((word >> 8) & 0xFF) as f32) / PACK_SCALE_8BIT,
        (((word >> 16) & 0xFF) as f32) / PACK_SCALE_8BIT,
        (((word >> 24) & 0xFF) as f32) / PACK_SCALE_8BIT,
    ]
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + ((end - start) * t.clamp(0.0, 1.0))
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    const Z: Self = Self::new(0.0, 0.0, 1.0);

    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    const fn from_array(values: [f32; 3]) -> Self {
        Self::new(values[0], values[1], values[2])
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
        let fallback = if self.z.abs() < 0.92 { Self::Z } else { Self::new(1.0, 0.0, 0.0) };
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

    fn compact_report_snapshot(report: &RedwoodForestFoliageReport) -> serde_json::Value {
        serde_json::json!({
            "seed_hex": report.seed_hex,
            "tree_count": report.tree_count,
            "total_tip_clusters": report.total_tip_clusters,
            "trees_truncated": report.trees_truncated,
            "within_budget": report.within_budget,
            "lods": report.lods.iter().map(|lod| {
                serde_json::json!({
                    "lod": lod.lod.as_str(),
                    "cluster_count": lod.cluster_count,
                    "card_count": lod.card_count,
                    "vertex_count": lod.vertex_count,
                    "triangle_count": lod.triangle_count,
                    "estimated_draw_calls": lod.estimated_draw_calls,
                    "mean_cluster_radius_m": format!("{:.3}", lod.mean_cluster_radius_m),
                    "max_card_half_extent_m": [
                        format!("{:.3}", lod.max_card_half_extent_m[0]),
                        format!("{:.3}", lod.max_card_half_extent_m[1]),
                    ],
                })
            }).collect::<Vec<_>>(),
            "trees": report.trees.iter().take(3).map(|tree| {
                serde_json::json!({
                    "tree_index": tree.tree_index,
                    "tip_cluster_count": tree.tip_cluster_count,
                    "lods": tree.lods.iter().map(|lod| serde_json::json!({
                        "lod": lod.lod.as_str(),
                        "card_count": lod.card_count,
                        "vertex_count": lod.vertex_count,
                    })).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
        })
    }

    #[test]
    fn foliage_report_matches_snapshot() {
        let graphs = canonical_graphs("0xDEADBEEF");
        let foliage = RedwoodForestFoliageSet::build(&graphs, RedwoodFoliageBuildConfig::default())
            .expect("foliage should build");

        assert_json_snapshot!(compact_report_snapshot(&foliage.report()), @r#"
        {
          "lods": [
            {
              "card_count": 5620,
              "cluster_count": 1124,
              "estimated_draw_calls": 1,
              "lod": "hero",
              "max_card_half_extent_m": [
                "0.823",
                "1.484"
              ],
              "mean_cluster_radius_m": "2.344",
              "triangle_count": 11240,
              "vertex_count": 22480
            },
            {
              "card_count": 4496,
              "cluster_count": 1124,
              "estimated_draw_calls": 1,
              "lod": "mid",
              "max_card_half_extent_m": [
                "0.887",
                "1.603"
              ],
              "mean_cluster_radius_m": "2.344",
              "triangle_count": 8992,
              "vertex_count": 17984
            },
            {
              "card_count": 3372,
              "cluster_count": 1124,
              "estimated_draw_calls": 1,
              "lod": "far",
              "max_card_half_extent_m": [
                "0.952",
                "1.721"
              ],
              "mean_cluster_radius_m": "2.344",
              "triangle_count": 6744,
              "vertex_count": 13488
            }
          ],
          "seed_hex": "0x00000000DEADBEEF",
          "total_tip_clusters": 1124,
          "tree_count": 275,
          "trees": [
            {
              "lods": [
                {
                  "card_count": 20,
                  "lod": "hero",
                  "vertex_count": 80
                },
                {
                  "card_count": 16,
                  "lod": "mid",
                  "vertex_count": 64
                },
                {
                  "card_count": 12,
                  "lod": "far",
                  "vertex_count": 48
                }
              ],
              "tip_cluster_count": 4,
              "tree_index": 0
            },
            {
              "lods": [
                {
                  "card_count": 20,
                  "lod": "hero",
                  "vertex_count": 80
                },
                {
                  "card_count": 16,
                  "lod": "mid",
                  "vertex_count": 64
                },
                {
                  "card_count": 12,
                  "lod": "far",
                  "vertex_count": 48
                }
              ],
              "tip_cluster_count": 4,
              "tree_index": 1
            },
            {
              "lods": [
                {
                  "card_count": 20,
                  "lod": "hero",
                  "vertex_count": 80
                },
                {
                  "card_count": 16,
                  "lod": "mid",
                  "vertex_count": 64
                },
                {
                  "card_count": 12,
                  "lod": "far",
                  "vertex_count": 48
                }
              ],
              "tip_cluster_count": 4,
              "tree_index": 2
            }
          ],
          "trees_truncated": true,
          "within_budget": true
        }
        "#);
    }

    #[test]
    fn material_pack_round_trip_stays_within_quantization() {
        let params = RedwoodFoliageMaterialParams {
            alpha_power: 0.72,
            edge_softness: 0.31,
            normal_bend: 0.56,
            backlight_boost: 0.62,
            hue_shift: -0.12,
            canopy_height_t: 0.88,
        };

        let unpacked = RedwoodFoliageMaterialParams::unpack(params.pack());

        assert!((params.alpha_power - unpacked.alpha_power).abs() < 0.01);
        assert!((params.edge_softness - unpacked.edge_softness).abs() < 0.01);
        assert!((params.normal_bend - unpacked.normal_bend).abs() < 0.01);
        assert!((params.backlight_boost - unpacked.backlight_boost).abs() < 0.01);
        assert!((params.hue_shift - unpacked.hue_shift).abs() < 0.01);
        assert!((params.canopy_height_t - unpacked.canopy_height_t).abs() < 0.01);
    }

    proptest! {
        #[test]
        fn foliage_cards_stay_finite(seed in any::<u64>()) {
            let root_seed = RootSeed::parse_hex(&format!("0x{seed:016X}"))
                .expect("seed should convert into canonical hex");
            let fields = TerrainScalarFieldSet::generate(root_seed, TerrainFieldConfig::default())
                .expect("fields should generate");
            let placements = EcologicalPlacementSet::generate(
                root_seed,
                &fields,
                EcologicalPlacementConfig::default(),
            )
            .expect("placements should generate");
            let graphs = RedwoodForestGraphSet::generate(
                root_seed,
                &fields,
                &placements,
                RedwoodForestGraphConfig::default(),
            )
            .expect("graphs should generate");
            let foliage = RedwoodForestFoliageSet::build(
                &graphs,
                RedwoodFoliageBuildConfig::default(),
            )
            .expect("foliage should build");
            let foliage_again = RedwoodForestFoliageSet::build(
                &graphs,
                RedwoodFoliageBuildConfig::default(),
            )
            .expect("foliage should deterministically rebuild");

            prop_assert_eq!(foliage.report(), foliage_again.report());

            for tree in foliage.trees() {
                for lod in RedwoodMeshLodTier::ALL {
                    let report = tree.lod(lod).report;
                    prop_assert!(report.card_count >= report.cluster_count * 2);
                    for cluster in &tree.lod(lod).clusters {
                        prop_assert!(cluster.envelope_radius_m.is_finite());
                        for card in &cluster.cards {
                            for vector in [card.center, card.axis_u, card.axis_v, card.normal] {
                                prop_assert!(vector.iter().all(|component| component.is_finite()));
                            }
                            prop_assert!(card.half_extents_m[0].is_finite() && card.half_extents_m[0] > 0.0);
                            prop_assert!(card.half_extents_m[1].is_finite() && card.half_extents_m[1] > 0.0);
                        }
                    }
                }
            }
        }
    }
}
