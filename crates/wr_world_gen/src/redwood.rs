use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{Level, debug};
use wr_math::{Vec2, clamp01, lerp};
use wr_world_seed::{RootSeed, stable_hash_u64_bytes};

use crate::{
    EcologicalPlacementKind, EcologicalPlacementSet, HERO_BIOME_HEIGHT_METERS,
    HERO_BIOME_WIDTH_METERS, TerrainScalarFieldSet,
};

const SILHOUETTE_GLYPHS: &[u8] = b" .:-=+*#%@";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodForestGraphConfig {
    pub width_m: f32,
    pub height_m: f32,
    pub attraction_radius_m: f32,
    pub kill_radius_m: f32,
    pub segment_length_m: f32,
    pub trunk_segment_length_m: f32,
    pub upward_tropism: f32,
    pub radial_tropism: f32,
    pub taper_decay: f32,
    pub min_radius_m: f32,
    pub buttress_depth: u8,
    pub buttress_radius_scale: f32,
    pub branch_cull_height_ratio: f32,
    pub canopy_center_height_ratio: f32,
    pub canopy_vertical_span_ratio: f32,
    pub canopy_radius_m: f32,
    pub canopy_radius_variance_m: f32,
    pub attractors_per_tree: u16,
    pub max_iterations: u16,
    pub debug_render_width: u16,
    pub debug_render_height: u16,
}

impl RedwoodForestGraphConfig {
    pub fn validate(self) -> Result<Self, RedwoodForestGraphError> {
        for (name, value) in [
            ("width_m", self.width_m),
            ("height_m", self.height_m),
            ("attraction_radius_m", self.attraction_radius_m),
            ("kill_radius_m", self.kill_radius_m),
            ("segment_length_m", self.segment_length_m),
            ("trunk_segment_length_m", self.trunk_segment_length_m),
            ("taper_decay", self.taper_decay),
            ("min_radius_m", self.min_radius_m),
            ("buttress_radius_scale", self.buttress_radius_scale),
            ("canopy_radius_m", self.canopy_radius_m),
            ("canopy_radius_variance_m", self.canopy_radius_variance_m),
        ] {
            if value.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) || !value.is_finite() {
                return Err(RedwoodForestGraphError::invalid_config(format!(
                    "{name} must be finite and positive"
                )));
            }
        }

        for (name, value) in [
            ("upward_tropism", self.upward_tropism),
            ("radial_tropism", self.radial_tropism),
            ("branch_cull_height_ratio", self.branch_cull_height_ratio),
            ("canopy_center_height_ratio", self.canopy_center_height_ratio),
            ("canopy_vertical_span_ratio", self.canopy_vertical_span_ratio),
        ] {
            if !(0.0..=1.0).contains(&value) || !value.is_finite() {
                return Err(RedwoodForestGraphError::invalid_config(format!(
                    "{name} must be finite and stay in [0, 1]"
                )));
            }
        }

        if self.kill_radius_m >= self.attraction_radius_m {
            return Err(RedwoodForestGraphError::invalid_config(
                "kill_radius_m must stay below attraction_radius_m",
            ));
        }
        if self.attractors_per_tree < 8 {
            return Err(RedwoodForestGraphError::invalid_config(
                "attractors_per_tree must be at least 8",
            ));
        }
        if self.attractors_per_tree > 1024 {
            return Err(RedwoodForestGraphError::invalid_config(
                "attractors_per_tree must stay at or below 1024 for the current growth pass",
            ));
        }
        if self.max_iterations < 4 {
            return Err(RedwoodForestGraphError::invalid_config(
                "max_iterations must be at least 4",
            ));
        }
        if self.max_iterations > 256 {
            return Err(RedwoodForestGraphError::invalid_config(
                "max_iterations must stay at or below 256 for the current growth pass",
            ));
        }
        if self.debug_render_width < 8 || self.debug_render_height < 8 {
            return Err(RedwoodForestGraphError::invalid_config(
                "debug render dimensions must be at least 8x8",
            ));
        }

        Ok(self)
    }
}

impl Default for RedwoodForestGraphConfig {
    fn default() -> Self {
        Self {
            width_m: HERO_BIOME_WIDTH_METERS,
            height_m: HERO_BIOME_HEIGHT_METERS,
            attraction_radius_m: 8.5,
            kill_radius_m: 2.8,
            segment_length_m: 2.6,
            trunk_segment_length_m: 3.1,
            upward_tropism: 0.82,
            radial_tropism: 0.16,
            taper_decay: 1.22,
            min_radius_m: 0.16,
            buttress_depth: 3,
            buttress_radius_scale: 1.55,
            branch_cull_height_ratio: 0.52,
            canopy_center_height_ratio: 0.8,
            canopy_vertical_span_ratio: 0.22,
            canopy_radius_m: 10.5,
            canopy_radius_variance_m: 4.0,
            attractors_per_tree: 84,
            max_iterations: 36,
            debug_render_width: 56,
            debug_render_height: 32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodForestGraphSet {
    seed_hex: String,
    config: RedwoodForestGraphConfig,
    trees: Vec<RedwoodTreeGraph>,
    report: RedwoodForestGraphReport,
}

impl RedwoodForestGraphSet {
    pub fn generate(
        root_seed: RootSeed,
        fields: &TerrainScalarFieldSet,
        placements: &EcologicalPlacementSet,
        config: RedwoodForestGraphConfig,
    ) -> Result<Self, RedwoodForestGraphError> {
        let config = config.validate()?;
        // Timing is debug-only observability and never feeds deterministic outputs.
        let started = tracing::enabled!(Level::DEBUG).then(Instant::now);
        let field_config = fields.config();
        let placement_config = placements.config();
        if !approx_eq(config.width_m, field_config.width_m)
            || !approx_eq(config.height_m, field_config.height_m)
            || !approx_eq(config.width_m, placement_config.width_m)
            || !approx_eq(config.height_m, placement_config.height_m)
        {
            return Err(RedwoodForestGraphError::invalid_config(
                "forest graph dimensions must match terrain and ecological placement dimensions",
            ));
        }

        let trunk_placements = placements.placements_for_kind(EcologicalPlacementKind::Trunk);
        let mut trees = Vec::with_capacity(trunk_placements.len());
        for (tree_index, placement) in trunk_placements.into_iter().enumerate() {
            let terrain_sample = fields.sample(placement.position());
            trees.push(generate_tree_graph(
                root_seed,
                tree_index,
                TreeGrowthInputs {
                    trunk_position: placement.position(),
                    ground_height_m: terrain_sample.height_m,
                    placement_score: placement.suitability,
                    orientation_radians: placement.orientation_radians,
                    moisture: terrain_sample.moisture,
                    fog: terrain_sample.fog,
                    canopy_opportunity: terrain_sample.canopy_opportunity,
                },
                &config,
            ));
        }

        let report = RedwoodForestGraphReport::from_trees(root_seed, config, &trees);
        if let Some(started) = started {
            debug!(
                seed_hex = %root_seed.to_hex(),
                tree_count = trees.len(),
                total_nodes = report.total_nodes,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                "generated redwood forest graph set",
            );
        }
        Ok(Self { seed_hex: root_seed.to_hex(), config, trees, report })
    }

    pub fn seed_hex(&self) -> &str {
        &self.seed_hex
    }

    pub fn config(&self) -> RedwoodForestGraphConfig {
        self.config
    }

    pub fn trees(&self) -> &[RedwoodTreeGraph] {
        &self.trees
    }

    pub fn summary_report(&self) -> RedwoodForestGraphReport {
        self.report.clone()
    }

    pub fn debug_dump(&self) -> RedwoodForestGraphDebugDump {
        let selected_trees =
            self.trees.iter().take(3).map(RedwoodTreeDebug::from_tree).collect::<Vec<_>>();

        RedwoodForestGraphDebugDump {
            seed_hex: self.seed_hex.clone(),
            width: self.config.debug_render_width,
            height: self.config.debug_render_height,
            report: self.summary_report(),
            front_view: render_silhouette(
                &self.trees,
                self.config.debug_render_width,
                self.config.debug_render_height,
                ProjectionPlane::Front,
            ),
            side_view: render_silhouette(
                &self.trees,
                self.config.debug_render_width,
                self.config.debug_render_height,
                ProjectionPlane::Side,
            ),
            selected_trees,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodTreeGraph {
    pub tree_index: usize,
    pub root_position: RedwoodPoint3,
    pub total_height_m: f32,
    pub canopy_center_height_m: f32,
    pub canopy_radius_m: f32,
    pub nodes: Vec<RedwoodNode>,
}

impl RedwoodTreeGraph {
    pub fn edge_count(&self) -> usize {
        self.nodes.iter().filter(|node| node.parent_index.is_some()).count()
    }

    pub fn tip_count(&self) -> usize {
        self.nodes.iter().filter(|node| node.children.is_empty()).count()
    }

    pub fn max_radius_m(&self) -> f32 {
        self.nodes.iter().map(|node| node.radius_m).fold(0.0, f32::max)
    }

    pub fn max_lateral_span_m(&self) -> f32 {
        let mut max_span: f32 = 0.0;
        for node in &self.nodes {
            let dx = node.position.x_m - self.root_position.x_m;
            let dy = node.position.y_m - self.root_position.y_m;
            max_span = max_span.max(((dx * dx) + (dy * dy)).sqrt());
        }
        max_span
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodNode {
    pub id: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_index: Option<usize>,
    /// Distance from the root in graph hops. Trunk nodes form the initial chain, so their segment
    /// index and hop depth are intentionally the same in the bootstrap implementation.
    pub depth: u16,
    pub position: RedwoodPoint3,
    pub radius_m: f32,
    pub children: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RedwoodPoint3 {
    pub x_m: f32,
    pub y_m: f32,
    pub z_m: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodForestGraphReport {
    pub seed_hex: String,
    pub width_m: f32,
    pub height_m: f32,
    pub tree_count: usize,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub mean_nodes_per_tree: f32,
    pub mean_tips_per_tree: f32,
    pub mean_height_m: f32,
    pub max_height_m: f32,
    pub mean_root_radius_m: f32,
    pub max_lateral_span_m: f32,
    pub trees: Vec<RedwoodTreeSummary>,
}

impl RedwoodForestGraphReport {
    fn from_trees(
        root_seed: RootSeed,
        config: RedwoodForestGraphConfig,
        trees: &[RedwoodTreeGraph],
    ) -> Self {
        let tree_count = trees.len();
        let total_nodes = trees.iter().map(|tree| tree.nodes.len()).sum::<usize>();
        let total_edges = trees.iter().map(RedwoodTreeGraph::edge_count).sum::<usize>();
        let mean_nodes_per_tree =
            if tree_count == 0 { 0.0 } else { total_nodes as f32 / tree_count as f32 };
        let mean_tips_per_tree = if tree_count == 0 {
            0.0
        } else {
            trees.iter().map(RedwoodTreeGraph::tip_count).sum::<usize>() as f32 / tree_count as f32
        };
        let mean_height_m = if tree_count == 0 {
            0.0
        } else {
            trees.iter().map(|tree| tree.total_height_m).sum::<f32>() / tree_count as f32
        };
        let max_height_m = trees.iter().map(|tree| tree.total_height_m).fold(0.0, f32::max);
        let mean_root_radius_m = if tree_count == 0 {
            0.0
        } else {
            trees.iter().map(|tree| tree.nodes[0].radius_m).sum::<f32>() / tree_count as f32
        };
        let max_lateral_span_m =
            trees.iter().map(RedwoodTreeGraph::max_lateral_span_m).fold(0.0, f32::max);

        Self {
            seed_hex: root_seed.to_hex(),
            width_m: config.width_m,
            height_m: config.height_m,
            tree_count,
            total_nodes,
            total_edges,
            mean_nodes_per_tree,
            mean_tips_per_tree,
            mean_height_m,
            max_height_m,
            mean_root_radius_m,
            max_lateral_span_m,
            trees: trees.iter().take(12).map(RedwoodTreeSummary::from_tree).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodTreeSummary {
    pub tree_index: usize,
    pub root_x_m: f32,
    pub root_y_m: f32,
    pub total_height_m: f32,
    pub node_count: usize,
    pub tip_count: usize,
    pub max_radius_m: f32,
    pub max_lateral_span_m: f32,
}

impl RedwoodTreeSummary {
    fn from_tree(tree: &RedwoodTreeGraph) -> Self {
        Self {
            tree_index: tree.tree_index,
            root_x_m: tree.root_position.x_m,
            root_y_m: tree.root_position.y_m,
            total_height_m: tree.total_height_m,
            node_count: tree.nodes.len(),
            tip_count: tree.tip_count(),
            max_radius_m: tree.max_radius_m(),
            max_lateral_span_m: tree.max_lateral_span_m(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodForestGraphDebugDump {
    pub seed_hex: String,
    pub width: u16,
    pub height: u16,
    pub report: RedwoodForestGraphReport,
    pub front_view: Vec<String>,
    pub side_view: Vec<String>,
    pub selected_trees: Vec<RedwoodTreeDebug>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodTreeDebug {
    pub tree_index: usize,
    pub node_count: usize,
    pub tip_count: usize,
    pub total_height_m: f32,
    pub selected_nodes: Vec<RedwoodDebugNode>,
}

impl RedwoodTreeDebug {
    fn from_tree(tree: &RedwoodTreeGraph) -> Self {
        let mut selected_nodes = Vec::new();
        let trunk_path = dominant_path(tree);
        for &node_index in trunk_path.iter().step_by(2).take(6) {
            let node = &tree.nodes[node_index];
            selected_nodes.push(RedwoodDebugNode::from_node(tree.root_position, node));
        }
        for node in tree.nodes.iter().rev().filter(|node| node.children.is_empty()).take(4) {
            selected_nodes.push(RedwoodDebugNode::from_node(tree.root_position, node));
        }

        Self {
            tree_index: tree.tree_index,
            node_count: tree.nodes.len(),
            tip_count: tree.tip_count(),
            total_height_m: tree.total_height_m,
            selected_nodes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedwoodDebugNode {
    pub depth: u16,
    pub x_offset_m: f32,
    pub y_offset_m: f32,
    pub z_m: f32,
    pub radius_m: f32,
}

impl RedwoodDebugNode {
    fn from_node(root: RedwoodPoint3, node: &RedwoodNode) -> Self {
        Self {
            depth: node.depth,
            x_offset_m: node.position.x_m - root.x_m,
            y_offset_m: node.position.y_m - root.y_m,
            z_m: node.position.z_m,
            radius_m: node.radius_m,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedwoodForestGraphError {
    reason: String,
}

impl RedwoodForestGraphError {
    fn invalid_config(reason: impl Into<String>) -> Self {
        Self { reason: reason.into() }
    }
}

impl std::fmt::Display for RedwoodForestGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for RedwoodForestGraphError {}

#[derive(Debug, Clone, Copy)]
struct RedwoodAttractor {
    position: RedwoodVec3,
}

#[derive(Debug, Clone, Copy, Default)]
struct Influence {
    direction_sums: [RedwoodVec3; 4],
    counts: [u16; 4],
}

#[derive(Debug, Clone, Copy)]
struct TreeGrowthInputs {
    trunk_position: Vec2,
    ground_height_m: f32,
    placement_score: f32,
    orientation_radians: f32,
    moisture: f32,
    fog: f32,
    canopy_opportunity: f32,
}

#[derive(Debug, Clone, Copy)]
struct CanopyShape {
    ground_height_m: f32,
    total_height_m: f32,
    center_height_m: f32,
    vertical_radius_m: f32,
    radius_m: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct RedwoodVec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl RedwoodVec3 {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn length(self) -> f32 {
        ((self.x * self.x) + (self.y * self.y) + (self.z * self.z)).sqrt()
    }

    fn normalize(self) -> Self {
        let length = self.length();
        if length <= f32::EPSILON {
            Self::new(0.0, 0.0, 1.0)
        } else {
            Self::new(self.x / length, self.y / length, self.z / length)
        }
    }
}

impl std::ops::Add for RedwoodVec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for RedwoodVec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for RedwoodVec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

fn generate_tree_graph(
    root_seed: RootSeed,
    tree_index: usize,
    inputs: TreeGrowthInputs,
    config: &RedwoodForestGraphConfig,
) -> RedwoodTreeGraph {
    let tree_seed = tree_seed(root_seed, tree_index);
    let total_height_m = lerp(
        62.0,
        104.0,
        clamp01((inputs.placement_score * 0.55) + (inputs.canopy_opportunity * 0.45)),
    );
    let canopy = CanopyShape {
        ground_height_m: inputs.ground_height_m,
        total_height_m,
        center_height_m: total_height_m * config.canopy_center_height_ratio,
        vertical_radius_m: total_height_m * config.canopy_vertical_span_ratio,
        radius_m: config.canopy_radius_m
            + (unit_hash01(tree_seed, "canopy.radius", 0) * config.canopy_radius_variance_m)
            + (inputs.canopy_opportunity * 3.0)
            + (inputs.moisture * 1.5),
    };
    let root_position = RedwoodPoint3 {
        x_m: inputs.trunk_position.x,
        y_m: inputs.trunk_position.y,
        z_m: inputs.ground_height_m,
    };
    let root_vec = RedwoodVec3::new(root_position.x_m, root_position.y_m, root_position.z_m);
    let trunk_target_height = total_height_m * lerp(0.62, 0.74, inputs.canopy_opportunity);
    let trunk_lean = RedwoodVec3::new(
        inputs.orientation_radians.cos() * 0.08,
        inputs.orientation_radians.sin() * 0.08,
        1.0,
    )
    .normalize();

    let mut nodes = vec![RedwoodNode {
        id: 0,
        parent_index: None,
        depth: 0,
        position: root_position,
        radius_m: config.min_radius_m,
        children: Vec::new(),
    }];

    let trunk_segments = (trunk_target_height / config.trunk_segment_length_m).ceil() as usize;
    let mut parent_index = 0;
    for segment in 0..trunk_segments {
        let growth = config.trunk_segment_length_m * (segment as f32 + 1.0);
        let lateral_scale = (segment as f32 / trunk_segments.max(1) as f32) * 0.35;
        let position = root_vec + (trunk_lean * growth);
        let new_node = RedwoodNode {
            id: nodes.len(),
            parent_index: Some(parent_index),
            depth: depth_from_index(segment + 1),
            position: RedwoodPoint3 {
                x_m: root_position.x_m + ((position.x - root_position.x_m) * lateral_scale),
                y_m: root_position.y_m + ((position.y - root_position.y_m) * lateral_scale),
                z_m: inputs.ground_height_m + growth,
            },
            radius_m: config.min_radius_m,
            children: Vec::new(),
        };
        let new_index = new_node.id;
        nodes[parent_index].children.push(new_index);
        nodes.push(new_node);
        parent_index = new_index;
    }

    let attractors =
        generate_attractors(tree_seed, inputs.trunk_position, canopy, inputs.fog, config);
    grow_canopy(
        root_position,
        total_height_m,
        canopy.center_height_m,
        canopy.radius_m,
        attractors,
        &mut nodes,
        config,
    );
    seed_crown_flares(
        tree_seed,
        root_position,
        total_height_m,
        canopy.radius_m,
        &mut nodes,
        config,
    );
    assign_radii(&mut nodes, config);

    RedwoodTreeGraph {
        tree_index,
        root_position,
        total_height_m,
        canopy_center_height_m: canopy.center_height_m,
        canopy_radius_m: canopy.radius_m,
        nodes,
    }
}

fn generate_attractors(
    tree_seed: RootSeed,
    trunk_position: Vec2,
    canopy: CanopyShape,
    fog: f32,
    config: &RedwoodForestGraphConfig,
) -> Vec<RedwoodAttractor> {
    let mut attractors = Vec::with_capacity(usize::from(config.attractors_per_tree));
    let lower_branch_cutoff =
        canopy.ground_height_m + (canopy.total_height_m * config.branch_cull_height_ratio);
    let elongated = lerp(0.8, 1.2, fog);
    for index in 0..usize::from(config.attractors_per_tree) {
        let angle = unit_hash01(tree_seed, "attractor.angle", index as u32) * std::f32::consts::TAU;
        let radius =
            canopy.radius_m * unit_hash01(tree_seed, "attractor.radius", index as u32).sqrt();
        let x = trunk_position.x + (angle.cos() * radius);
        let y = trunk_position.y + (angle.sin() * radius * lerp(0.82, 1.12, fog));
        let vertical = ((unit_hash01(tree_seed, "attractor.height", index as u32) * 2.0) - 1.0)
            * canopy.vertical_radius_m
            * elongated;
        let z = (canopy.ground_height_m + canopy.center_height_m + vertical)
            .clamp(lower_branch_cutoff, canopy.ground_height_m + canopy.total_height_m);
        attractors.push(RedwoodAttractor { position: RedwoodVec3::new(x, y, z) });
    }
    attractors
}

fn grow_canopy(
    root_position: RedwoodPoint3,
    total_height_m: f32,
    canopy_center_height_m: f32,
    canopy_radius_m: f32,
    mut attractors: Vec<RedwoodAttractor>,
    nodes: &mut Vec<RedwoodNode>,
    config: &RedwoodForestGraphConfig,
) {
    let branch_cutoff = root_position.z_m + (total_height_m * config.branch_cull_height_ratio);

    for _ in 0..config.max_iterations {
        if attractors.is_empty() {
            break;
        }

        let mut influences = vec![Influence::default(); nodes.len()];
        let mut survivors = Vec::with_capacity(attractors.len());

        for attractor in attractors {
            let mut nearest_index = None;
            let mut nearest_distance_sq = config.attraction_radius_m * config.attraction_radius_m;
            let mut consumed = false;

            for (node_index, node) in nodes.iter().enumerate() {
                let node_position =
                    RedwoodVec3::new(node.position.x_m, node.position.y_m, node.position.z_m);
                let offset = attractor.position - node_position;
                let distance_sq =
                    (offset.x * offset.x) + (offset.y * offset.y) + (offset.z * offset.z);
                if distance_sq <= config.kill_radius_m * config.kill_radius_m {
                    // Any node inside the kill radius is enough to consume the attractor; this is
                    // intentionally "any-node kill" rather than "nearest-node kill".
                    consumed = true;
                    break;
                }
                if distance_sq < nearest_distance_sq {
                    nearest_distance_sq = distance_sq;
                    nearest_index = Some(node_index);
                }
            }

            if consumed {
                continue;
            }

            if let Some(node_index) = nearest_index {
                let node = &nodes[node_index];
                let node_position =
                    RedwoodVec3::new(node.position.x_m, node.position.y_m, node.position.z_m);
                let outward = RedwoodVec3::new(
                    node.position.x_m - root_position.x_m,
                    node.position.y_m - root_position.y_m,
                    0.0,
                )
                .normalize();
                let upward = RedwoodVec3::new(0.0, 0.0, config.upward_tropism);
                let canopy_pull = RedwoodVec3::new(
                    0.0,
                    0.0,
                    clamp01(
                        (root_position.z_m + canopy_center_height_m - node.position.z_m)
                            / total_height_m,
                    ),
                );
                let direction = (attractor.position - node_position).normalize()
                    + upward
                    + canopy_pull
                    + (outward * config.radial_tropism);
                let bin = branch_bin(attractor.position - node_position);
                influences[node_index].direction_sums[bin] =
                    influences[node_index].direction_sums[bin] + direction;
                influences[node_index].counts[bin] += 1;
                survivors.push(attractor);
            }
        }

        let mut new_nodes = Vec::new();
        for (node_index, influence) in influences.iter().enumerate() {
            let node = &nodes[node_index];
            let parent_position =
                RedwoodVec3::new(node.position.x_m, node.position.y_m, node.position.z_m);
            for bin in 0..influence.counts.len() {
                if influence.counts[bin] == 0 {
                    continue;
                }
                let direction = influence.direction_sums[bin].normalize();
                let next_position = parent_position + (direction * config.segment_length_m);
                let clamped = RedwoodVec3::new(
                    next_position.x,
                    next_position.y,
                    next_position.z.min(root_position.z_m + total_height_m),
                );
                if clamped.z <= branch_cutoff {
                    continue;
                }
                if too_close_to_existing(nodes, clamped, config.segment_length_m * 0.72) {
                    continue;
                }
                let radial_dx = clamped.x - root_position.x_m;
                let radial_dy = clamped.y - root_position.y_m;
                let radial_distance = ((radial_dx * radial_dx) + (radial_dy * radial_dy)).sqrt();
                if radial_distance > canopy_radius_m * 1.2 {
                    continue;
                }

                new_nodes.push((node_index, clamped));
            }
        }

        if new_nodes.is_empty() {
            break;
        }

        for (parent_index, position) in new_nodes {
            let new_index = nodes.len();
            let parent_depth = nodes[parent_index].depth;
            nodes[parent_index].children.push(new_index);
            nodes.push(RedwoodNode {
                id: new_index,
                parent_index: Some(parent_index),
                depth: parent_depth + 1,
                position: RedwoodPoint3 { x_m: position.x, y_m: position.y, z_m: position.z },
                radius_m: config.min_radius_m,
                children: Vec::new(),
            });
        }

        attractors = survivors;
    }
}

fn assign_radii(nodes: &mut [RedwoodNode], config: &RedwoodForestGraphConfig) {
    for index in (0..nodes.len()).rev() {
        let child_radii =
            nodes[index].children.iter().map(|&child| nodes[child].radius_m).collect::<Vec<_>>();
        let mut radius = if child_radii.is_empty() {
            config.min_radius_m
        } else {
            let max_child = child_radii.iter().copied().fold(0.0, f32::max);
            let branch_sum = child_radii.iter().map(|radius| radius * radius).sum::<f32>().sqrt();
            (branch_sum / config.taper_decay).max(max_child + (config.min_radius_m * 0.18))
        };

        if usize::from(nodes[index].depth) <= usize::from(config.buttress_depth) {
            let t = if config.buttress_depth == 0 {
                1.0
            } else {
                nodes[index].depth as f32 / f32::from(config.buttress_depth)
            };
            radius *= lerp(config.buttress_radius_scale, 1.0, clamp01(t));
        }

        nodes[index].radius_m = radius.max(config.min_radius_m);
    }
}

fn seed_crown_flares(
    tree_seed: RootSeed,
    root_position: RedwoodPoint3,
    total_height_m: f32,
    canopy_radius_m: f32,
    nodes: &mut Vec<RedwoodNode>,
    config: &RedwoodForestGraphConfig,
) {
    let branch_cutoff = root_position.z_m + (total_height_m * config.branch_cull_height_ratio);
    let trunk_path = dominant_path_from_nodes(nodes);
    let anchors = trunk_path
        .into_iter()
        .filter(|&index| nodes[index].position.z_m >= branch_cutoff)
        .rev()
        .step_by(2)
        .take(4)
        .collect::<Vec<_>>();

    for (anchor_order, anchor_index) in anchors.into_iter().enumerate() {
        let anchor = nodes[anchor_index].clone();
        let anchor_position =
            RedwoodVec3::new(anchor.position.x_m, anchor.position.y_m, anchor.position.z_m);
        for side in 0..2 {
            let sample_index = (anchor_order * 2 + side) as u32;
            let angle = unit_hash01(tree_seed, "flare.angle", sample_index) * std::f32::consts::TAU;
            let length = lerp(
                canopy_radius_m * 0.28,
                canopy_radius_m * 0.52,
                unit_hash01(tree_seed, "flare.length", sample_index),
            );
            let rise = lerp(0.18, 0.42, unit_hash01(tree_seed, "flare.rise", sample_index));
            let outward = RedwoodVec3::new(angle.cos(), angle.sin(), rise).normalize();
            let child_position = anchor_position + (outward * length);
            let clamped = RedwoodVec3::new(
                child_position.x,
                child_position.y,
                child_position.z.clamp(
                    branch_cutoff + (config.segment_length_m * 0.25),
                    root_position.z_m + total_height_m,
                ),
            );
            if too_close_to_existing(nodes, clamped, config.segment_length_m * 0.72) {
                continue;
            }

            let child_index = push_child(nodes, anchor_index, clamped);
            let taper_length =
                length * lerp(0.35, 0.55, unit_hash01(tree_seed, "flare.tip", sample_index));
            let tip_position = clamped + (outward * taper_length);
            let tip_clamped = RedwoodVec3::new(
                tip_position.x,
                tip_position.y,
                tip_position.z.clamp(
                    branch_cutoff + (config.segment_length_m * 0.25),
                    root_position.z_m + total_height_m,
                ),
            );
            if too_close_to_existing(nodes, tip_clamped, config.segment_length_m * 0.72) {
                continue;
            }
            let _ = push_child(nodes, child_index, tip_clamped);
        }
    }
}

fn push_child(nodes: &mut Vec<RedwoodNode>, parent_index: usize, position: RedwoodVec3) -> usize {
    let child_index = nodes.len();
    let depth = nodes[parent_index].depth.saturating_add(1);
    nodes[parent_index].children.push(child_index);
    nodes.push(RedwoodNode {
        id: child_index,
        parent_index: Some(parent_index),
        depth,
        position: RedwoodPoint3 { x_m: position.x, y_m: position.y, z_m: position.z },
        radius_m: 0.0,
        children: Vec::new(),
    });
    child_index
}

fn too_close_to_existing(
    nodes: &[RedwoodNode],
    candidate: RedwoodVec3,
    min_distance_m: f32,
) -> bool {
    let min_distance_sq = min_distance_m * min_distance_m;
    nodes.iter().any(|node| {
        let dx = candidate.x - node.position.x_m;
        let dy = candidate.y - node.position.y_m;
        let dz = candidate.z - node.position.z_m;
        ((dx * dx) + (dy * dy) + (dz * dz)) < min_distance_sq
    })
}

fn branch_bin(offset: RedwoodVec3) -> usize {
    let angle = offset.y.atan2(offset.x);
    let normalized = (angle + std::f32::consts::PI) / std::f32::consts::TAU;
    ((normalized * 4.0).floor() as usize) % 4
}

fn tree_seed(root_seed: RootSeed, tree_index: usize) -> RootSeed {
    let mut bytes = root_seed.derive_stream_u64("trees.redwood").to_be_bytes().to_vec();
    bytes.extend_from_slice(&(tree_index as u64).to_be_bytes());
    let hash = stable_hash_u64_bytes(bytes);
    RootSeed::parse_hex(&format!("0x{hash:016X}")).expect("stable hash should produce a valid seed")
}

fn unit_hash01(seed: RootSeed, label: &str, index: u32) -> f32 {
    let mut bytes = seed.derive_stream_u64(label).to_be_bytes().to_vec();
    bytes.extend_from_slice(&index.to_be_bytes());
    let hash = stable_hash_u64_bytes(bytes);
    // We intentionally keep the inclusive `[0, 1]` mapping because sibling generators in this
    // repo do the same top-24-bit normalization for deterministic stability.
    ((hash >> 40) as f32) / ((1_u64 << 24) - 1) as f32
}

fn depth_from_index(index: usize) -> u16 {
    index.min(usize::from(u16::MAX)) as u16
}

fn dominant_path(tree: &RedwoodTreeGraph) -> Vec<usize> {
    dominant_path_from_nodes(&tree.nodes)
}

fn dominant_path_from_nodes(nodes: &[RedwoodNode]) -> Vec<usize> {
    let mut path = vec![0];
    let mut current = 0;
    while let Some(&child) = nodes[current]
        .children
        .iter()
        .max_by(|left, right| nodes[**left].position.z_m.total_cmp(&nodes[**right].position.z_m))
    {
        path.push(child);
        current = child;
    }
    path
}

#[derive(Debug, Clone, Copy)]
enum ProjectionPlane {
    Front,
    Side,
}

#[derive(Debug, Clone, Copy)]
struct RenderProjection {
    width: u16,
    height: u16,
    max_height: f32,
    max_span: f32,
    plane: ProjectionPlane,
}

fn render_silhouette(
    trees: &[RedwoodTreeGraph],
    width: u16,
    height: u16,
    plane: ProjectionPlane,
) -> Vec<String> {
    let mut counts = vec![0_u16; usize::from(width) * usize::from(height)];
    let max_height =
        trees.iter().map(|tree| tree.root_position.z_m + tree.total_height_m).fold(1.0, f32::max);
    let max_span = trees.iter().map(|tree| tree.max_lateral_span_m()).fold(1.0, f32::max).max(4.0);
    let projection = RenderProjection { width, height, max_height, max_span, plane };

    for tree in trees.iter().take(48) {
        for node in &tree.nodes {
            if let Some(parent_index) = node.parent_index {
                let parent = &tree.nodes[parent_index];
                rasterize_segment(
                    parent.position,
                    node.position,
                    tree.root_position,
                    projection,
                    &mut counts,
                );
            }
        }
    }

    let max_count = counts.iter().copied().max().unwrap_or(0).max(1);
    let mut rows = Vec::with_capacity(usize::from(height));
    for row in 0..usize::from(height) {
        let mut line = String::with_capacity(usize::from(width));
        for column in 0..usize::from(width) {
            let count = counts[row * usize::from(width) + column];
            let glyph_index = usize::from(
                ((count as f32 / max_count as f32) * (SILHOUETTE_GLYPHS.len() - 1) as f32).round()
                    as u16,
            );
            line.push(SILHOUETTE_GLYPHS[glyph_index.min(SILHOUETTE_GLYPHS.len() - 1)] as char);
        }
        rows.push(line);
    }
    rows
}

fn rasterize_segment(
    start: RedwoodPoint3,
    end: RedwoodPoint3,
    root: RedwoodPoint3,
    projection: RenderProjection,
    counts: &mut [u16],
) {
    let steps = 6;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let x = lerp(start.x_m, end.x_m, t);
        let y = lerp(start.y_m, end.y_m, t);
        let z = lerp(start.z_m, end.z_m, t);
        let lateral = match projection.plane {
            ProjectionPlane::Front => x - root.x_m,
            ProjectionPlane::Side => y - root.y_m,
        };
        let normalized_x = clamp01((lateral + projection.max_span) / (projection.max_span * 2.0));
        let normalized_y = 1.0 - clamp01(z / projection.max_height);
        let column = ((normalized_x * f32::from(projection.width.saturating_sub(1))).round()
            as usize)
            .min(usize::from(projection.width.saturating_sub(1)));
        let row = ((normalized_y * f32::from(projection.height.saturating_sub(1))).round()
            as usize)
            .min(usize::from(projection.height.saturating_sub(1)));
        let index = row * usize::from(projection.width) + column;
        counts[index] = counts[index].saturating_add(1);
    }
}

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.01
}

#[cfg(test)]
mod tests {
    use insta::assert_json_snapshot;
    use proptest::prelude::*;
    use serde_json::json;

    use super::*;
    use crate::TerrainFieldConfig;

    fn canonical_field_config() -> TerrainFieldConfig {
        TerrainFieldConfig { cache_resolution: 65, ..TerrainFieldConfig::default() }
    }

    fn canonical_graph_config() -> RedwoodForestGraphConfig {
        RedwoodForestGraphConfig::default()
    }

    fn root_seed_from_u64(seed: u64) -> RootSeed {
        RootSeed::parse_hex(&format!("0x{seed:016X}")).expect("generated seed should parse")
    }

    fn generate_set(seed: RootSeed) -> RedwoodForestGraphSet {
        let fields =
            TerrainScalarFieldSet::generate(seed, canonical_field_config()).expect("field set");
        let placements = EcologicalPlacementSet::generate(
            seed,
            &fields,
            crate::EcologicalPlacementConfig::default(),
        )
        .expect("placements");
        RedwoodForestGraphSet::generate(seed, &fields, &placements, canonical_graph_config())
            .expect("redwood graph set")
    }

    fn compact_debug_snapshot(dump: &RedwoodForestGraphDebugDump) -> serde_json::Value {
        json!({
            "seed_hex": dump.seed_hex,
            "tree_count": dump.report.tree_count,
            "front_view": dump.front_view.iter().take(12).collect::<Vec<_>>(),
            "side_view": dump.side_view.iter().take(12).collect::<Vec<_>>(),
            "selected_trees": dump.selected_trees.iter().take(2).map(|tree| {
                json!({
                    "tree_index": tree.tree_index,
                    "node_count": tree.node_count,
                    "tip_count": tree.tip_count,
                    "total_height_m": fmt3(tree.total_height_m),
                    "selected_nodes": tree.selected_nodes.iter().take(6).map(|node| {
                        json!({
                            "depth": node.depth,
                            "x_offset_m": fmt3(node.x_offset_m),
                            "y_offset_m": fmt3(node.y_offset_m),
                            "z_m": fmt3(node.z_m),
                            "radius_m": fmt3(node.radius_m),
                        })
                    }).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn compact_report_snapshot(report: &RedwoodForestGraphReport) -> serde_json::Value {
        json!({
            "seed_hex": report.seed_hex,
            "tree_count": report.tree_count,
            "total_nodes": report.total_nodes,
            "total_edges": report.total_edges,
            "mean_nodes_per_tree": fmt3(report.mean_nodes_per_tree),
            "mean_tips_per_tree": fmt3(report.mean_tips_per_tree),
            "mean_height_m": fmt3(report.mean_height_m),
            "max_height_m": fmt3(report.max_height_m),
            "mean_root_radius_m": fmt3(report.mean_root_radius_m),
            "max_lateral_span_m": fmt3(report.max_lateral_span_m),
            "trees": report.trees.iter().take(4).map(|tree| {
                json!({
                    "tree_index": tree.tree_index,
                    "root_x_m": fmt3(tree.root_x_m),
                    "root_y_m": fmt3(tree.root_y_m),
                    "total_height_m": fmt3(tree.total_height_m),
                    "node_count": tree.node_count,
                    "tip_count": tree.tip_count,
                    "max_radius_m": fmt3(tree.max_radius_m),
                    "max_lateral_span_m": fmt3(tree.max_lateral_span_m),
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn fmt3(value: f32) -> String {
        format!("{value:.3}")
    }

    #[test]
    fn hero_forest_graph_report_matches_snapshot() {
        let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let report = generate_set(seed).summary_report();

        assert_json_snapshot!(compact_report_snapshot(&report), @r#"
        {
          "max_height_m": "97.866",
          "max_lateral_span_m": "19.462",
          "mean_height_m": "94.591",
          "mean_nodes_per_tree": "31.636",
          "mean_root_radius_m": "2.124",
          "mean_tips_per_tree": "4.087",
          "seed_hex": "0x00000000DEADBEEF",
          "total_edges": 8425,
          "total_nodes": 8700,
          "tree_count": 275,
          "trees": [
            {
              "max_lateral_span_m": "10.495",
              "max_radius_m": "2.167",
              "node_count": 32,
              "root_x_m": "14.413",
              "root_y_m": "98.413",
              "tip_count": 4,
              "total_height_m": "97.866",
              "tree_index": 0
            },
            {
              "max_lateral_span_m": "8.699",
              "max_radius_m": "2.167",
              "node_count": 32,
              "root_x_m": "4.337",
              "root_y_m": "88.337",
              "tip_count": 4,
              "total_height_m": "97.569",
              "tree_index": 1
            },
            {
              "max_lateral_span_m": "10.268",
              "max_radius_m": "2.167",
              "node_count": 32,
              "root_x_m": "6.834",
              "root_y_m": "114.834",
              "tip_count": 4,
              "total_height_m": "97.356",
              "tree_index": 2
            },
            {
              "max_lateral_span_m": "7.441",
              "max_radius_m": "2.167",
              "node_count": 32,
              "root_x_m": "105.017",
              "root_y_m": "9.017",
              "tip_count": 4,
              "total_height_m": "97.388",
              "tree_index": 3
            }
          ]
        }
        "#);
    }

    #[test]
    fn duel_focus_debug_dump_matches_snapshot() {
        let seed = RootSeed::parse_hex("0xC0FFEE01").expect("seed should parse");
        let dump = generate_set(seed).debug_dump();

        assert_json_snapshot!(compact_debug_snapshot(&dump), @r#"
        {
          "front_view": [
            "                                                        ",
            "                                                        ",
            "                                                        ",
            "                                                        ",
            "                                                        ",
            "                                                        ",
            "                                                        ",
            "                     ..  ..                             ",
            "                   ......=-::::..                       ",
            "                   ......-+-:--..                       ",
            "                    .....:*--=:..   .                   ",
            "                         .+=-=.                         "
          ],
          "seed_hex": "0x00000000C0FFEE01",
          "selected_trees": [
            {
              "node_count": 32,
              "selected_nodes": [
                {
                  "depth": 0,
                  "radius_m": "2.167",
                  "x_offset_m": "0.000",
                  "y_offset_m": "0.000",
                  "z_m": "58.826"
                },
                {
                  "depth": 2,
                  "radius_m": "0.973",
                  "x_offset_m": "-0.001",
                  "y_offset_m": "0.007",
                  "z_m": "65.026"
                },
                {
                  "depth": 4,
                  "radius_m": "0.765",
                  "x_offset_m": "-0.009",
                  "y_offset_m": "0.044",
                  "z_m": "71.226"
                },
                {
                  "depth": 6,
                  "radius_m": "0.707",
                  "x_offset_m": "-0.022",
                  "y_offset_m": "0.111",
                  "z_m": "77.426"
                },
                {
                  "depth": 8,
                  "radius_m": "0.650",
                  "x_offset_m": "-0.041",
                  "y_offset_m": "0.207",
                  "z_m": "83.626"
                },
                {
                  "depth": 10,
                  "radius_m": "0.592",
                  "x_offset_m": "-0.066",
                  "y_offset_m": "0.332",
                  "z_m": "89.826"
                }
              ],
              "tip_count": 4,
              "total_height_m": "96.200",
              "tree_index": 0
            },
            {
              "node_count": 32,
              "selected_nodes": [
                {
                  "depth": 0,
                  "radius_m": "2.167",
                  "x_offset_m": "0.000",
                  "y_offset_m": "0.000",
                  "z_m": "64.218"
                },
                {
                  "depth": 2,
                  "radius_m": "0.973",
                  "x_offset_m": "0.004",
                  "y_offset_m": "-0.006",
                  "z_m": "70.418"
                },
                {
                  "depth": 4,
                  "radius_m": "0.765",
                  "x_offset_m": "0.025",
                  "y_offset_m": "-0.037",
                  "z_m": "76.618"
                },
                {
                  "depth": 6,
                  "radius_m": "0.707",
                  "x_offset_m": "0.063",
                  "y_offset_m": "-0.094",
                  "z_m": "82.818"
                },
                {
                  "depth": 8,
                  "radius_m": "0.650",
                  "x_offset_m": "0.117",
                  "y_offset_m": "-0.175",
                  "z_m": "89.018"
                },
                {
                  "depth": 10,
                  "radius_m": "0.592",
                  "x_offset_m": "0.189",
                  "y_offset_m": "-0.281",
                  "z_m": "95.218"
                }
              ],
              "tip_count": 4,
              "total_height_m": "96.206",
              "tree_index": 1
            }
          ],
          "side_view": [
            "                                                        ",
            "                                                        ",
            "                                                        ",
            "                                                        ",
            "                                                        ",
            "                                                        ",
            "                                                        ",
            "                          .  .:.                        ",
            "                    ..   :-..=-...                      ",
            "                         :-::+-.... .                   ",
            "                         .=-=+-......                   ",
            "                          --*=. .                       "
          ],
          "tree_count": 275
        }
        "#);
    }

    #[test]
    fn deterministic_generation_rebuilds_identical_graphs() {
        let seed = RootSeed::parse_hex("0xF00DFACE").expect("seed should parse");
        let first = generate_set(seed);
        let second = generate_set(seed);

        assert_eq!(first, second);
    }

    #[test]
    fn invalid_config_is_rejected() {
        let invalid = [
            RedwoodForestGraphConfig { width_m: 0.0, ..RedwoodForestGraphConfig::default() },
            RedwoodForestGraphConfig {
                kill_radius_m: 8.5,
                attraction_radius_m: 8.0,
                ..RedwoodForestGraphConfig::default()
            },
            RedwoodForestGraphConfig {
                attractors_per_tree: 4,
                ..RedwoodForestGraphConfig::default()
            },
            RedwoodForestGraphConfig {
                debug_render_width: 6,
                ..RedwoodForestGraphConfig::default()
            },
        ];

        for config in invalid {
            assert!(config.validate().is_err());
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(16))]

        #[test]
        fn generated_graphs_are_connected_and_acyclic(seed in any::<u64>()) {
            let set = generate_set(root_seed_from_u64(seed));

            for tree in set.trees() {
                prop_assert!(!tree.nodes.is_empty());
                prop_assert_eq!(tree.nodes[0].parent_index, None);
                for (index, node) in tree.nodes.iter().enumerate() {
                    if let Some(parent_index) = node.parent_index {
                        prop_assert!(parent_index < index);
                    } else {
                        prop_assert_eq!(index, 0);
                    }
                }
                prop_assert_eq!(tree.edge_count() + 1, tree.nodes.len());
            }
        }

        #[test]
        fn radii_taper_monotonically_away_from_root(seed in any::<u64>()) {
            let set = generate_set(root_seed_from_u64(seed));

            for tree in set.trees() {
                for node in &tree.nodes {
                    if let Some(parent_index) = node.parent_index {
                        let parent = &tree.nodes[parent_index];
                        prop_assert!(parent.radius_m + 0.0001 >= node.radius_m);
                    }
                }
            }
        }

        #[test]
        fn canopy_tips_stay_above_cull_height(seed in any::<u64>()) {
            let set = generate_set(root_seed_from_u64(seed));

            for tree in set.trees() {
                let cutoff = tree.root_position.z_m + (tree.total_height_m * canonical_graph_config().branch_cull_height_ratio);
                for node in tree.nodes.iter().filter(|node| node.children.is_empty()) {
                    let lateral_offset = ((node.position.x_m - tree.root_position.x_m).powi(2)
                        + (node.position.y_m - tree.root_position.y_m).powi(2)).sqrt();
                    if lateral_offset > 0.75 {
                        prop_assert!(node.position.z_m >= cutoff - 0.001);
                    }
                }
            }
        }
    }
}
