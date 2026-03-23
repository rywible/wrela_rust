use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use wr_math::{Vec2, clamp01};
use wr_world_seed::{RootSeed, stable_hash_u64_bytes};

use crate::{
    HERO_BIOME_HEIGHT_METERS, HERO_BIOME_WIDTH_METERS, TerrainFieldSample, TerrainScalarFieldSet,
};
const DEBUG_MAP_INTENSITY_STEP: u16 = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EcologicalPlacementKind {
    Trunk,
    Understory,
    DeadfallAnchor,
}

impl EcologicalPlacementKind {
    pub const ALL: [Self; 3] = [Self::Trunk, Self::Understory, Self::DeadfallAnchor];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trunk => "trunk",
            Self::Understory => "understory",
            Self::DeadfallAnchor => "deadfall_anchor",
        }
    }

    const fn as_index(self) -> usize {
        match self {
            Self::Trunk => 0,
            Self::Understory => 1,
            Self::DeadfallAnchor => 2,
        }
    }

    const fn score_label(self) -> &'static str {
        match self {
            Self::Trunk => "ecology.trunk.score",
            Self::Understory => "ecology.understory.score",
            Self::DeadfallAnchor => "ecology.deadfall.score",
        }
    }

    const fn jitter_label(self) -> &'static str {
        match self {
            Self::Trunk => "ecology.trunk.jitter",
            Self::Understory => "ecology.understory.jitter",
            Self::DeadfallAnchor => "ecology.deadfall.jitter",
        }
    }

    const fn orientation_label(self) -> &'static str {
        match self {
            Self::Trunk => "ecology.trunk.orientation",
            Self::Understory => "ecology.understory.orientation",
            Self::DeadfallAnchor => "ecology.deadfall.orientation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EcologicalPlacementConfig {
    pub width_m: f32,
    pub height_m: f32,
    pub trunk_density_per_hectare: f32,
    pub understory_density_per_hectare: f32,
    pub deadfall_density_per_hectare: f32,
    pub trunk_min_spacing_m: f32,
    pub understory_min_spacing_m: f32,
    pub deadfall_min_spacing_m: f32,
    pub understory_trunk_clearance_m: f32,
    pub deadfall_trunk_clearance_m: f32,
    pub forbidden_path_threshold: f32,
    pub max_trunk_slope: f32,
    pub max_understory_slope: f32,
    pub max_deadfall_slope: f32,
    pub candidate_jitter_ratio: f32,
    pub debug_map_resolution: u16,
}

impl EcologicalPlacementConfig {
    pub fn validate(self) -> Result<Self, EcologicalPlacementError> {
        if self.width_m.partial_cmp(&0.0) != Some(Ordering::Greater) {
            return Err(EcologicalPlacementError::invalid_config("width_m must be positive"));
        }
        if self.height_m.partial_cmp(&0.0) != Some(Ordering::Greater) {
            return Err(EcologicalPlacementError::invalid_config("height_m must be positive"));
        }
        if self.trunk_density_per_hectare.partial_cmp(&0.0) != Some(Ordering::Greater) {
            return Err(EcologicalPlacementError::invalid_config(
                "trunk_density_per_hectare must be positive",
            ));
        }
        if self.understory_density_per_hectare.partial_cmp(&0.0) != Some(Ordering::Greater) {
            return Err(EcologicalPlacementError::invalid_config(
                "understory_density_per_hectare must be positive",
            ));
        }
        if self.deadfall_density_per_hectare.partial_cmp(&0.0) != Some(Ordering::Greater) {
            return Err(EcologicalPlacementError::invalid_config(
                "deadfall_density_per_hectare must be positive",
            ));
        }
        for (name, spacing) in [
            ("trunk_min_spacing_m", self.trunk_min_spacing_m),
            ("understory_min_spacing_m", self.understory_min_spacing_m),
            ("deadfall_min_spacing_m", self.deadfall_min_spacing_m),
            ("understory_trunk_clearance_m", self.understory_trunk_clearance_m),
            ("deadfall_trunk_clearance_m", self.deadfall_trunk_clearance_m),
        ] {
            if spacing.partial_cmp(&0.0) != Some(Ordering::Greater) {
                return Err(EcologicalPlacementError::invalid_config(format!(
                    "{name} must be positive"
                )));
            }
        }
        for (name, ratio) in [
            ("forbidden_path_threshold", self.forbidden_path_threshold),
            ("max_trunk_slope", self.max_trunk_slope),
            ("max_understory_slope", self.max_understory_slope),
            ("max_deadfall_slope", self.max_deadfall_slope),
            ("candidate_jitter_ratio", self.candidate_jitter_ratio),
        ] {
            if !(0.0..=1.0).contains(&ratio) || !ratio.is_finite() {
                return Err(EcologicalPlacementError::invalid_config(format!(
                    "{name} must be finite and stay in [0, 1]"
                )));
            }
        }
        if self.debug_map_resolution < 2 {
            return Err(EcologicalPlacementError::invalid_config(
                "debug_map_resolution must be at least 2",
            ));
        }

        Ok(self)
    }

    fn spec(self, kind: EcologicalPlacementKind) -> PlacementSpec {
        match kind {
            EcologicalPlacementKind::Trunk => PlacementSpec {
                kind,
                density_per_hectare: self.trunk_density_per_hectare,
                min_spacing_m: self.trunk_min_spacing_m,
                cross_kind_clearance_to_trunks_m: None,
                max_slope: self.max_trunk_slope,
                forbid_hero_corridor: true,
            },
            EcologicalPlacementKind::Understory => PlacementSpec {
                kind,
                density_per_hectare: self.understory_density_per_hectare,
                min_spacing_m: self.understory_min_spacing_m,
                cross_kind_clearance_to_trunks_m: Some(self.understory_trunk_clearance_m),
                max_slope: self.max_understory_slope,
                forbid_hero_corridor: false,
            },
            EcologicalPlacementKind::DeadfallAnchor => PlacementSpec {
                kind,
                density_per_hectare: self.deadfall_density_per_hectare,
                min_spacing_m: self.deadfall_min_spacing_m,
                cross_kind_clearance_to_trunks_m: Some(self.deadfall_trunk_clearance_m),
                max_slope: self.max_deadfall_slope,
                forbid_hero_corridor: true,
            },
        }
    }
}

impl Default for EcologicalPlacementConfig {
    fn default() -> Self {
        Self {
            width_m: HERO_BIOME_WIDTH_METERS,
            height_m: HERO_BIOME_HEIGHT_METERS,
            trunk_density_per_hectare: 10.5,
            understory_density_per_hectare: 32.0,
            deadfall_density_per_hectare: 7.5,
            trunk_min_spacing_m: 12.0,
            understory_min_spacing_m: 5.5,
            deadfall_min_spacing_m: 14.0,
            understory_trunk_clearance_m: 4.0,
            deadfall_trunk_clearance_m: 6.0,
            forbidden_path_threshold: 0.62,
            max_trunk_slope: 0.52,
            max_understory_slope: 0.72,
            max_deadfall_slope: 0.68,
            candidate_jitter_ratio: 0.35,
            debug_map_resolution: 32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EcologicalPlacement {
    pub kind: EcologicalPlacementKind,
    pub x_m: f32,
    pub y_m: f32,
    pub suitability: f32,
    pub orientation_radians: f32,
    pub sampled_height_m: f32,
    pub sampled_slope: f32,
    pub sampled_drainage: f32,
    pub sampled_moisture: f32,
    pub sampled_fog: f32,
    pub sampled_canopy_opportunity: f32,
    pub sampled_deadfall_probability: f32,
    pub sampled_hero_path_bias: f32,
}

impl EcologicalPlacement {
    pub fn position(&self) -> Vec2 {
        Vec2::new(self.x_m, self.y_m)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EcologicalPlacementSet {
    seed_hex: String,
    config: EcologicalPlacementConfig,
    placements: Vec<EcologicalPlacement>,
    report: EcologicalPlacementReport,
}

impl EcologicalPlacementSet {
    pub fn generate(
        root_seed: RootSeed,
        fields: &TerrainScalarFieldSet,
        config: EcologicalPlacementConfig,
    ) -> Result<Self, EcologicalPlacementError> {
        let config = config.validate()?;
        let field_config = fields.config();
        if !approx_eq(config.width_m, field_config.width_m)
            || !approx_eq(config.height_m, field_config.height_m)
        {
            return Err(EcologicalPlacementError::invalid_config(
                "placement config dimensions must match the terrain field dimensions",
            ));
        }

        let mut accepted_by_kind = [Vec::new(), Vec::new(), Vec::new()];
        let mut summaries = Vec::with_capacity(EcologicalPlacementKind::ALL.len());

        for kind in EcologicalPlacementKind::ALL {
            let spec = config.spec(kind);
            let candidates = generate_candidates(root_seed, fields, config, spec);
            let solve = select_candidates(
                config,
                spec,
                &candidates,
                &accepted_by_kind[EcologicalPlacementKind::Trunk.as_index()],
            );

            for placement in &solve.accepted {
                accepted_by_kind[kind.as_index()].push(placement.clone());
            }

            summaries.push(build_summary(config, spec, &solve.accepted, solve.stats));
        }

        let placements = EcologicalPlacementKind::ALL
            .into_iter()
            .flat_map(|kind| accepted_by_kind[kind.as_index()].iter().cloned())
            .collect::<Vec<_>>();

        let report = EcologicalPlacementReport {
            seed_hex: root_seed.to_hex(),
            width_m: config.width_m,
            height_m: config.height_m,
            debug_map_resolution: config.debug_map_resolution,
            forbidden_path_threshold: config.forbidden_path_threshold,
            summaries,
        };

        Ok(Self { seed_hex: root_seed.to_hex(), config, placements, report })
    }

    pub fn seed_hex(&self) -> &str {
        &self.seed_hex
    }

    pub fn config(&self) -> EcologicalPlacementConfig {
        self.config
    }

    pub fn placements(&self) -> &[EcologicalPlacement] {
        &self.placements
    }

    pub fn placements_for_kind(&self, kind: EcologicalPlacementKind) -> Vec<&EcologicalPlacement> {
        self.placements.iter().filter(|placement| placement.kind == kind).collect()
    }

    pub fn summary_report(&self) -> EcologicalPlacementReport {
        self.report.clone()
    }

    pub fn debug_dump(&self, fields: &TerrainScalarFieldSet) -> EcologicalPlacementDebugDump {
        let mut maps = BTreeMap::new();
        for kind in EcologicalPlacementKind::ALL {
            let mut map = vec![
                0_u8;
                usize::from(self.config.debug_map_resolution)
                    * usize::from(self.config.debug_map_resolution)
            ];
            for placement in self.placements.iter().filter(|placement| placement.kind == kind) {
                let index = quantized_map_index(
                    self.config.debug_map_resolution,
                    self.config.width_m,
                    self.config.height_m,
                    placement.position(),
                );
                let next = u16::from(map[index]) + DEBUG_MAP_INTENSITY_STEP;
                map[index] = next.min(255) as u8;
            }
            maps.insert(format!("{}_occupancy", kind.as_str()), map);
        }

        let mut forbidden_path = vec![
            0_u8;
            usize::from(self.config.debug_map_resolution)
                * usize::from(self.config.debug_map_resolution)
        ];
        for row in 0..self.config.debug_map_resolution {
            let v = row as f32 / (self.config.debug_map_resolution.saturating_sub(1) as f32);
            let y = self.config.height_m * v;
            for column in 0..self.config.debug_map_resolution {
                let u = column as f32 / (self.config.debug_map_resolution.saturating_sub(1) as f32);
                let x = self.config.width_m * u;
                let sample = fields.sample(Vec2::new(x, y));
                let index = usize::from(row) * usize::from(self.config.debug_map_resolution)
                    + usize::from(column);
                forbidden_path[index] =
                    if sample.hero_path_bias >= self.config.forbidden_path_threshold {
                        255
                    } else {
                        0
                    };
            }
        }
        maps.insert("forbidden_path_corridor".to_owned(), forbidden_path);

        EcologicalPlacementDebugDump {
            seed_hex: self.seed_hex.clone(),
            resolution: self.config.debug_map_resolution,
            report: self.summary_report(),
            maps,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EcologicalPlacementReport {
    pub seed_hex: String,
    pub width_m: f32,
    pub height_m: f32,
    pub debug_map_resolution: u16,
    pub forbidden_path_threshold: f32,
    pub summaries: Vec<EcologicalPlacementSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EcologicalPlacementSummary {
    pub kind: EcologicalPlacementKind,
    pub target_count: usize,
    pub accepted_count: usize,
    pub fill_ratio: f32,
    pub mean_suitability: f32,
    pub mean_sampled_slope: f32,
    pub mean_sampled_drainage: f32,
    pub mean_sampled_moisture: f32,
    pub mean_sampled_fog: f32,
    pub mean_sampled_canopy_opportunity: f32,
    pub mean_sampled_deadfall_probability: f32,
    pub mean_sampled_hero_path_bias: f32,
    pub mean_nearest_spacing_m: f32,
    pub min_nearest_spacing_m: f32,
    pub rejected_forbidden_path: usize,
    pub rejected_slope: usize,
    pub rejected_same_kind_spacing: usize,
    pub rejected_trunk_competition: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EcologicalPlacementDebugDump {
    pub seed_hex: String,
    pub resolution: u16,
    pub report: EcologicalPlacementReport,
    pub maps: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcologicalPlacementError {
    reason: String,
}

impl EcologicalPlacementError {
    fn invalid_config(reason: impl Into<String>) -> Self {
        Self { reason: reason.into() }
    }
}

impl std::fmt::Display for EcologicalPlacementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for EcologicalPlacementError {}

#[derive(Debug, Clone, Copy)]
struct PlacementSpec {
    kind: EcologicalPlacementKind,
    density_per_hectare: f32,
    min_spacing_m: f32,
    cross_kind_clearance_to_trunks_m: Option<f32>,
    max_slope: f32,
    forbid_hero_corridor: bool,
}

#[derive(Debug, Clone)]
struct PlacementCandidate {
    placement: EcologicalPlacement,
    tie_breaker: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct RejectionStats {
    forbidden_path: usize,
    slope: usize,
    same_kind_spacing: usize,
    trunk_competition: usize,
}

#[derive(Debug, Clone)]
struct PlacementSolve {
    accepted: Vec<EcologicalPlacement>,
    stats: RejectionStats,
}

fn generate_candidates(
    root_seed: RootSeed,
    fields: &TerrainScalarFieldSet,
    config: EcologicalPlacementConfig,
    spec: PlacementSpec,
) -> Vec<PlacementCandidate> {
    let cell_size = spec.min_spacing_m.max(1.0);
    let columns = (config.width_m / cell_size).ceil() as u32;
    let rows = (config.height_m / cell_size).ceil() as u32;
    let jitter_span = cell_size * config.candidate_jitter_ratio;
    let mut candidates = Vec::with_capacity((columns as usize) * (rows as usize));
    for row in 0..rows {
        for column in 0..columns {
            let center_x = ((column as f32) + 0.5) * cell_size;
            let center_y = ((row as f32) + 0.5) * cell_size;
            let jitter_x =
                signed_unit_hash(root_seed, spec.kind.jitter_label(), column, row, 0) * jitter_span;
            let jitter_y =
                signed_unit_hash(root_seed, spec.kind.jitter_label(), column, row, 1) * jitter_span;
            let point = Vec2::new(center_x + jitter_x, center_y + jitter_y)
                .clamp(Vec2::new(0.0, 0.0), Vec2::new(config.width_m, config.height_m));
            let sample = fields.sample(point);
            let suitability = score_candidate(spec.kind, sample).clamp(0.0, 1.0);
            let tie_breaker = hash_coordinate(root_seed, spec.kind.score_label(), column, row, 2);
            let orientation_radians =
                unit_hash01(root_seed, spec.kind.orientation_label(), column, row, 3)
                    * std::f32::consts::TAU;

            candidates.push(PlacementCandidate {
                placement: EcologicalPlacement {
                    kind: spec.kind,
                    x_m: point.x,
                    y_m: point.y,
                    suitability,
                    orientation_radians,
                    sampled_height_m: sample.height_m,
                    sampled_slope: sample.slope,
                    sampled_drainage: sample.drainage,
                    sampled_moisture: sample.moisture,
                    sampled_fog: sample.fog,
                    sampled_canopy_opportunity: sample.canopy_opportunity,
                    sampled_deadfall_probability: sample.deadfall_probability,
                    sampled_hero_path_bias: sample.hero_path_bias,
                },
                tie_breaker,
            });
        }
    }

    candidates.sort_by(|left, right| {
        right
            .placement
            .suitability
            .total_cmp(&left.placement.suitability)
            .then_with(|| left.tie_breaker.cmp(&right.tie_breaker))
    });
    candidates
}

fn select_candidates(
    config: EcologicalPlacementConfig,
    spec: PlacementSpec,
    candidates: &[PlacementCandidate],
    accepted_trunks: &[EcologicalPlacement],
) -> PlacementSolve {
    let target_count = target_count(spec, config.width_m, config.height_m);
    let mut accepted = Vec::with_capacity(target_count);
    let mut same_kind_index =
        PlacementSpatialIndex::new(config.width_m, config.height_m, spec.min_spacing_m);
    let mut trunk_index = PlacementSpatialIndex::new(
        config.width_m,
        config.height_m,
        spec.cross_kind_clearance_to_trunks_m.unwrap_or(spec.min_spacing_m),
    );
    for trunk in accepted_trunks {
        trunk_index.insert(trunk.position());
    }

    let mut stats = RejectionStats::default();

    for candidate in candidates {
        if accepted.len() >= target_count {
            break;
        }
        if spec.forbid_hero_corridor
            && candidate.placement.sampled_hero_path_bias >= config.forbidden_path_threshold
        {
            stats.forbidden_path += 1;
            continue;
        }
        if candidate.placement.sampled_slope > spec.max_slope {
            stats.slope += 1;
            continue;
        }
        if same_kind_index.has_neighbor_within(candidate.placement.position(), spec.min_spacing_m) {
            stats.same_kind_spacing += 1;
            continue;
        }
        if let Some(clearance_m) = spec.cross_kind_clearance_to_trunks_m
            && trunk_index.has_neighbor_within(candidate.placement.position(), clearance_m)
        {
            stats.trunk_competition += 1;
            continue;
        }

        same_kind_index.insert(candidate.placement.position());
        accepted.push(candidate.placement.clone());
    }

    PlacementSolve { accepted, stats }
}

fn build_summary(
    config: EcologicalPlacementConfig,
    spec: PlacementSpec,
    accepted: &[EcologicalPlacement],
    stats: RejectionStats,
) -> EcologicalPlacementSummary {
    let target_count = target_count(spec, config.width_m, config.height_m);
    let accepted_count = accepted.len();
    let mut mean_suitability = 0.0;
    let mut mean_sampled_slope = 0.0;
    let mut mean_sampled_drainage = 0.0;
    let mut mean_sampled_moisture = 0.0;
    let mut mean_sampled_fog = 0.0;
    let mut mean_sampled_canopy_opportunity = 0.0;
    let mut mean_sampled_deadfall_probability = 0.0;
    let mut mean_sampled_hero_path_bias = 0.0;

    for placement in accepted {
        mean_suitability += placement.suitability;
        mean_sampled_slope += placement.sampled_slope;
        mean_sampled_drainage += placement.sampled_drainage;
        mean_sampled_moisture += placement.sampled_moisture;
        mean_sampled_fog += placement.sampled_fog;
        mean_sampled_canopy_opportunity += placement.sampled_canopy_opportunity;
        mean_sampled_deadfall_probability += placement.sampled_deadfall_probability;
        mean_sampled_hero_path_bias += placement.sampled_hero_path_bias;
    }

    let normalization = (accepted_count.max(1)) as f32;
    let (mean_nearest_spacing_m, min_nearest_spacing_m) = nearest_spacing_stats(accepted);

    EcologicalPlacementSummary {
        kind: spec.kind,
        target_count,
        accepted_count,
        fill_ratio: if target_count == 0 {
            1.0
        } else {
            accepted_count as f32 / target_count as f32
        },
        mean_suitability: mean_suitability / normalization,
        mean_sampled_slope: mean_sampled_slope / normalization,
        mean_sampled_drainage: mean_sampled_drainage / normalization,
        mean_sampled_moisture: mean_sampled_moisture / normalization,
        mean_sampled_fog: mean_sampled_fog / normalization,
        mean_sampled_canopy_opportunity: mean_sampled_canopy_opportunity / normalization,
        mean_sampled_deadfall_probability: mean_sampled_deadfall_probability / normalization,
        mean_sampled_hero_path_bias: mean_sampled_hero_path_bias / normalization,
        mean_nearest_spacing_m,
        min_nearest_spacing_m,
        rejected_forbidden_path: stats.forbidden_path,
        rejected_slope: stats.slope,
        rejected_same_kind_spacing: stats.same_kind_spacing,
        rejected_trunk_competition: stats.trunk_competition,
    }
}

fn target_count(spec: PlacementSpec, width_m: f32, height_m: f32) -> usize {
    let area_hectares = (width_m * height_m) / 10_000.0;
    (spec.density_per_hectare * area_hectares).round() as usize
}

fn nearest_spacing_stats(placements: &[EcologicalPlacement]) -> (f32, f32) {
    if placements.len() < 2 {
        return (0.0, 0.0);
    }

    // Reports compute this once per solve, so an O(n^2) pass keeps the bootstrap implementation
    // simple and inspectable at current density targets.
    let mut total = 0.0;
    let mut min_spacing = f32::INFINITY;
    for (index, placement) in placements.iter().enumerate() {
        let mut nearest = f32::INFINITY;
        for (other_index, other) in placements.iter().enumerate() {
            if index == other_index {
                continue;
            }
            nearest = nearest.min(distance(placement.position(), other.position()));
        }
        total += nearest;
        min_spacing = min_spacing.min(nearest);
    }

    (total / placements.len() as f32, min_spacing)
}

fn score_candidate(kind: EcologicalPlacementKind, sample: TerrainFieldSample) -> f32 {
    let slope_support = 1.0 - sample.slope;
    let path_clearance = 1.0 - sample.hero_path_bias;
    match kind {
        EcologicalPlacementKind::Trunk => clamp01(
            (sample.canopy_opportunity * 0.44)
                + (sample.moisture * 0.22)
                + (sample.deadfall_probability * 0.05)
                + (slope_support * 0.19)
                + (path_clearance * 0.10),
        ),
        EcologicalPlacementKind::Understory => {
            let canopy_band = 1.0 - ((sample.canopy_opportunity - 0.58).abs() * 1.75);
            clamp01(
                (sample.moisture * 0.35)
                    + (sample.fog * 0.18)
                    + (clamp01(canopy_band) * 0.24)
                    + (slope_support * 0.11)
                    + (path_clearance * 0.12),
            )
        }
        EcologicalPlacementKind::DeadfallAnchor => clamp01(
            (sample.deadfall_probability * 0.43)
                + (sample.drainage * 0.18)
                + (sample.moisture * 0.08)
                + (sample.slope * 0.11)
                + ((1.0 - sample.canopy_opportunity) * 0.12)
                + (path_clearance * 0.08),
        ),
    }
}

fn quantized_map_index(resolution: u16, width_m: f32, height_m: f32, point: Vec2) -> usize {
    let clamped = point.clamp(Vec2::new(0.0, 0.0), Vec2::new(width_m, height_m));
    let max_index = f32::from(resolution.saturating_sub(1));
    let column = ((clamped.x / width_m) * max_index).round() as usize;
    let row = ((clamped.y / height_m) * max_index).round() as usize;
    row * usize::from(resolution) + column
}

fn unit_hash01(root_seed: RootSeed, label: &str, column: u32, row: u32, axis: u32) -> f32 {
    unit_interval_from_hash(hash_coordinate(root_seed, label, column, row, axis))
}

fn signed_unit_hash(root_seed: RootSeed, label: &str, column: u32, row: u32, axis: u32) -> f32 {
    (unit_hash01(root_seed, label, column, row, axis) * 2.0) - 1.0
}

fn hash_coordinate(root_seed: RootSeed, label: &str, column: u32, row: u32, axis: u32) -> u64 {
    let mut bytes = Vec::with_capacity(label.len() + 20);
    bytes.extend_from_slice(&root_seed.derive_stream_u64(label).to_be_bytes());
    bytes.extend_from_slice(&column.to_be_bytes());
    bytes.extend_from_slice(&row.to_be_bytes());
    bytes.extend_from_slice(&axis.to_be_bytes());
    stable_hash_u64_bytes(bytes)
}

fn unit_interval_from_hash(hash: u64) -> f32 {
    ((hash >> 40) as f32) / ((1_u64 << 24) - 1) as f32
}

fn distance(a: Vec2, b: Vec2) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    ((dx * dx) + (dy * dy)).sqrt()
}

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.01
}

#[derive(Debug, Clone)]
struct PlacementSpatialIndex {
    width_m: f32,
    height_m: f32,
    cell_size_m: f32,
    columns: usize,
    rows: usize,
    buckets: Vec<Vec<Vec2>>,
}

impl PlacementSpatialIndex {
    fn new(width_m: f32, height_m: f32, cell_size_m: f32) -> Self {
        let cell_size_m = cell_size_m.max(1.0);
        let columns = (width_m / cell_size_m).ceil() as usize + 1;
        let rows = (height_m / cell_size_m).ceil() as usize + 1;
        Self {
            width_m,
            height_m,
            cell_size_m,
            columns,
            rows,
            buckets: vec![Vec::new(); columns * rows],
        }
    }

    fn insert(&mut self, point: Vec2) {
        let bucket_index = self.bucket_index(point);
        self.buckets[bucket_index].push(point);
    }

    fn has_neighbor_within(&self, point: Vec2, radius_m: f32) -> bool {
        let cell_radius = (radius_m / self.cell_size_m).ceil() as i32;
        let (column, row) = self.bucket_coordinates(point);
        let max_column = self.columns.saturating_sub(1) as i32;
        let max_row = self.rows.saturating_sub(1) as i32;
        let radius_sq = radius_m * radius_m;

        for delta_row in -cell_radius..=cell_radius {
            let current_row = row + delta_row;
            if !(0..=max_row).contains(&current_row) {
                continue;
            }
            for delta_column in -cell_radius..=cell_radius {
                let current_column = column + delta_column;
                if !(0..=max_column).contains(&current_column) {
                    continue;
                }
                let bucket_index =
                    self.bucket_index_from_grid(current_column as usize, current_row as usize);
                if self.buckets[bucket_index].iter().any(|neighbor| {
                    let dx = point.x - neighbor.x;
                    let dy = point.y - neighbor.y;
                    ((dx * dx) + (dy * dy)) < radius_sq
                }) {
                    return true;
                }
            }
        }

        false
    }

    fn bucket_index(&self, point: Vec2) -> usize {
        let (column, row) = self.bucket_coordinates(point);
        self.bucket_index_from_grid(column as usize, row as usize)
    }

    fn bucket_coordinates(&self, point: Vec2) -> (i32, i32) {
        let clamped = point.clamp(Vec2::new(0.0, 0.0), Vec2::new(self.width_m, self.height_m));
        let max_column = self.columns.saturating_sub(1) as i32;
        let max_row = self.rows.saturating_sub(1) as i32;
        let column = ((clamped.x / self.cell_size_m).floor() as i32).clamp(0, max_column);
        let row = ((clamped.y / self.cell_size_m).floor() as i32).clamp(0, max_row);
        (column, row)
    }

    fn bucket_index_from_grid(&self, column: usize, row: usize) -> usize {
        let column = column.min(self.columns.saturating_sub(1));
        let row = row.min(self.rows.saturating_sub(1));
        row.saturating_mul(self.columns) + column
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use insta::assert_json_snapshot;
    use proptest::prelude::*;

    use crate::TerrainFieldConfig;

    use super::*;

    fn canonical_field_config() -> TerrainFieldConfig {
        TerrainFieldConfig { cache_resolution: 65, ..TerrainFieldConfig::default() }
    }

    fn canonical_placement_config() -> EcologicalPlacementConfig {
        EcologicalPlacementConfig::default()
    }

    fn root_seed_from_u64(seed: u64) -> RootSeed {
        RootSeed::parse_hex(&format!("0x{seed:016X}")).expect("generated seed should parse")
    }

    fn compact_debug_snapshot(dump: &EcologicalPlacementDebugDump) -> serde_json::Value {
        let occupancy_digest = dump
            .maps
            .iter()
            .map(|(name, values)| {
                let nonzero_cells = values.iter().filter(|value| **value > 0).count();
                let total_intensity = values.iter().map(|value| u64::from(*value)).sum::<u64>();
                let max_intensity = values.iter().copied().max().unwrap_or_default();
                (
                    name.clone(),
                    serde_json::json!({
                        "nonzero_cells": nonzero_cells,
                        "total_intensity": total_intensity,
                        "max_intensity": max_intensity
                    }),
                )
            })
            .collect::<BTreeMap<_, _>>();

        serde_json::json!({
            "seed_hex": dump.seed_hex,
            "resolution": dump.resolution,
            "map_digest": occupancy_digest
        })
    }

    fn generate_set(seed: RootSeed) -> EcologicalPlacementSet {
        let fields =
            TerrainScalarFieldSet::generate(seed, canonical_field_config()).expect("field set");
        EcologicalPlacementSet::generate(seed, &fields, canonical_placement_config())
            .expect("placement solve")
    }

    #[test]
    fn hero_forest_report_matches_snapshot() {
        let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let report = generate_set(seed).summary_report();

        assert_json_snapshot!(report, @r#"
        {
          "seed_hex": "0x00000000DEADBEEF",
          "width_m": 512.0,
          "height_m": 512.0,
          "debug_map_resolution": 32,
          "forbidden_path_threshold": 0.62,
          "summaries": [
            {
              "kind": "trunk",
              "target_count": 275,
              "accepted_count": 275,
              "fill_ratio": 1.0,
              "mean_suitability": 0.77081,
              "mean_sampled_slope": 0.07009226,
              "mean_sampled_drainage": 0.6060538,
              "mean_sampled_moisture": 0.58969194,
              "mean_sampled_fog": 0.5491599,
              "mean_sampled_canopy_opportunity": 0.7822945,
              "mean_sampled_deadfall_probability": 0.40611455,
              "mean_sampled_hero_path_bias": 0.0011992807,
              "mean_nearest_spacing_m": 15.748578,
              "min_nearest_spacing_m": 12.02947,
              "rejected_forbidden_path": 0,
              "rejected_slope": 0,
              "rejected_same_kind_spacing": 278,
              "rejected_trunk_competition": 0
            },
            {
              "kind": "understory",
              "target_count": 839,
              "accepted_count": 839,
              "fill_ratio": 1.0,
              "mean_suitability": 0.711784,
              "mean_sampled_slope": 0.11227583,
              "mean_sampled_drainage": 0.6235077,
              "mean_sampled_moisture": 0.6068769,
              "mean_sampled_fog": 0.5719839,
              "mean_sampled_canopy_opportunity": 0.7238746,
              "mean_sampled_deadfall_probability": 0.4413516,
              "mean_sampled_hero_path_bias": 0.006690436,
              "mean_nearest_spacing_m": 6.488406,
              "min_nearest_spacing_m": 5.5276413,
              "rejected_forbidden_path": 0,
              "rejected_slope": 0,
              "rejected_same_kind_spacing": 866,
              "rejected_trunk_competition": 96
            },
            {
              "kind": "deadfall_anchor",
              "target_count": 197,
              "accepted_count": 197,
              "fill_ratio": 1.0,
              "mean_suitability": 0.4776375,
              "mean_sampled_slope": 0.14982416,
              "mean_sampled_drainage": 0.59719825,
              "mean_sampled_moisture": 0.5815284,
              "mean_sampled_fog": 0.5433886,
              "mean_sampled_canopy_opportunity": 0.7226017,
              "mean_sampled_deadfall_probability": 0.45229352,
              "mean_sampled_hero_path_bias": 0.007940113,
              "mean_nearest_spacing_m": 18.391777,
              "min_nearest_spacing_m": 14.093929,
              "rejected_forbidden_path": 0,
              "rejected_slope": 0,
              "rejected_same_kind_spacing": 227,
              "rejected_trunk_competition": 18
            }
          ]
        }
        "#);
    }

    #[test]
    fn duel_focus_debug_dump_matches_snapshot() {
        let seed = RootSeed::parse_hex("0xC0FFEE01").expect("seed should parse");
        let fields =
            TerrainScalarFieldSet::generate(seed, canonical_field_config()).expect("field set");
        let placements =
            EcologicalPlacementSet::generate(seed, &fields, canonical_placement_config())
                .expect("placement solve");

        assert_json_snapshot!(compact_debug_snapshot(&placements.debug_dump(&fields)), @r#"
        {
          "map_digest": {
            "deadfall_anchor_occupancy": {
              "max_intensity": 96,
              "nonzero_cells": 196,
              "total_intensity": 9456
            },
            "forbidden_path_corridor": {
              "max_intensity": 255,
              "nonzero_cells": 127,
              "total_intensity": 32385
            },
            "trunk_occupancy": {
              "max_intensity": 144,
              "nonzero_cells": 252,
              "total_intensity": 13200
            },
            "understory_occupancy": {
              "max_intensity": 255,
              "nonzero_cells": 278,
              "total_intensity": 39486
            }
          },
          "resolution": 32,
          "seed_hex": "0x00000000C0FFEE01"
        }
        "#);
    }

    #[test]
    fn deterministic_generation_rebuilds_identical_placements() {
        let seed = RootSeed::parse_hex("0xF00DFACE").expect("seed should parse");
        let first = generate_set(seed);
        let second = generate_set(seed);

        assert_eq!(first, second);
    }

    #[test]
    fn invalid_config_is_rejected() {
        let invalid = [
            EcologicalPlacementConfig { width_m: 0.0, ..EcologicalPlacementConfig::default() },
            EcologicalPlacementConfig {
                trunk_density_per_hectare: 0.0,
                ..EcologicalPlacementConfig::default()
            },
            EcologicalPlacementConfig {
                forbidden_path_threshold: 1.2,
                ..EcologicalPlacementConfig::default()
            },
            EcologicalPlacementConfig {
                debug_map_resolution: 1,
                ..EcologicalPlacementConfig::default()
            },
        ];

        for config in invalid {
            assert!(config.validate().is_err());
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn major_placements_respect_forbidden_corridor_and_slope(seed in any::<u64>()) {
            let seed = root_seed_from_u64(seed);
            let fields = TerrainScalarFieldSet::generate(seed, canonical_field_config()).expect("field set");
            let placements = EcologicalPlacementSet::generate(seed, &fields, canonical_placement_config()).expect("placement solve");

            for placement in placements.placements() {
                match placement.kind {
                    EcologicalPlacementKind::Trunk => {
                        prop_assert!(placement.sampled_hero_path_bias < canonical_placement_config().forbidden_path_threshold);
                        prop_assert!(placement.sampled_slope <= canonical_placement_config().max_trunk_slope);
                    }
                    EcologicalPlacementKind::DeadfallAnchor => {
                        prop_assert!(placement.sampled_hero_path_bias < canonical_placement_config().forbidden_path_threshold);
                        prop_assert!(placement.sampled_slope <= canonical_placement_config().max_deadfall_slope);
                    }
                    EcologicalPlacementKind::Understory => {
                        prop_assert!(placement.sampled_slope <= canonical_placement_config().max_understory_slope);
                    }
                }
            }
        }

        #[test]
        fn same_kind_spacing_never_violates_config(seed in any::<u64>()) {
            let set = generate_set(root_seed_from_u64(seed));
            let config = canonical_placement_config();

            for kind in EcologicalPlacementKind::ALL {
                let min_spacing = match kind {
                    EcologicalPlacementKind::Trunk => config.trunk_min_spacing_m,
                    EcologicalPlacementKind::Understory => config.understory_min_spacing_m,
                    EcologicalPlacementKind::DeadfallAnchor => config.deadfall_min_spacing_m,
                };
                let placements = set.placements_for_kind(kind);
                for (index, placement) in placements.iter().enumerate() {
                    for other in placements.iter().skip(index + 1) {
                        let spacing = distance(placement.position(), other.position());
                        prop_assert!(spacing >= min_spacing - 0.001);
                    }
                }
            }
        }

        #[test]
        fn trunk_clearance_rules_hold_for_competing_kinds(seed in any::<u64>()) {
            let set = generate_set(root_seed_from_u64(seed));
            let config = canonical_placement_config();
            let trunks = set.placements_for_kind(EcologicalPlacementKind::Trunk);

            for placement in set.placements_for_kind(EcologicalPlacementKind::Understory) {
                for trunk in &trunks {
                    let spacing = distance(placement.position(), trunk.position());
                    prop_assert!(spacing >= config.understory_trunk_clearance_m - 0.001);
                }
            }

            for placement in set.placements_for_kind(EcologicalPlacementKind::DeadfallAnchor) {
                for trunk in &trunks {
                    let spacing = distance(placement.position(), trunk.position());
                    prop_assert!(spacing >= config.deadfall_trunk_clearance_m - 0.001);
                }
            }
        }
    }

    #[test]
    fn mismatched_field_dimensions_are_rejected() {
        let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let fields =
            TerrainScalarFieldSet::generate(seed, canonical_field_config()).expect("field set");
        let config = EcologicalPlacementConfig {
            width_m: HERO_BIOME_WIDTH_METERS - 1.0,
            ..canonical_placement_config()
        };

        assert!(EcologicalPlacementSet::generate(seed, &fields, config).is_err());
    }
}
