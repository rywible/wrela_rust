#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{Level, debug};
use wr_core::{CrateBoundary, CrateEntryPoint};
use wr_math::{FractalNoise2, Vec2, clamp01, inverse_lerp, lerp, smootherstep01};
use wr_world_seed::RootSeed;

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_world_gen", CrateBoundary::Subsystem, false)
}

pub const HERO_BIOME_WIDTH_METERS: f32 = 512.0;
pub const HERO_BIOME_HEIGHT_METERS: f32 = 512.0;

const FIELD_COUNT: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainFieldKind {
    Height,
    Slope,
    Drainage,
    Moisture,
    Fog,
    CanopyOpportunity,
    DeadfallProbability,
    HeroPathBias,
}

impl TerrainFieldKind {
    pub const ALL: [Self; FIELD_COUNT] = [
        Self::Height,
        Self::Slope,
        Self::Drainage,
        Self::Moisture,
        Self::Fog,
        Self::CanopyOpportunity,
        Self::DeadfallProbability,
        Self::HeroPathBias,
    ];

    pub const fn as_index(self) -> usize {
        match self {
            Self::Height => 0,
            Self::Slope => 1,
            Self::Drainage => 2,
            Self::Moisture => 3,
            Self::Fog => 4,
            Self::CanopyOpportunity => 5,
            Self::DeadfallProbability => 6,
            Self::HeroPathBias => 7,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Height => "height",
            Self::Slope => "slope",
            Self::Drainage => "drainage",
            Self::Moisture => "moisture",
            Self::Fog => "fog",
            Self::CanopyOpportunity => "canopy_opportunity",
            Self::DeadfallProbability => "deadfall_probability",
            Self::HeroPathBias => "hero_path_bias",
        }
    }
}

const _: [(); FIELD_COUNT] = [(); TerrainFieldKind::ALL.len()];

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainFieldConfig {
    pub width_m: f32,
    pub height_m: f32,
    pub cache_resolution: u16,
    pub height_base_m: f32,
    pub height_variation_m: f32,
    pub slope_probe_m: f32,
}

impl TerrainFieldConfig {
    pub fn validate(self) -> Result<Self, TerrainFieldError> {
        if self.width_m <= 0.0 {
            return Err(TerrainFieldError::invalid_config("width_m must be positive"));
        }
        if self.height_m <= 0.0 {
            return Err(TerrainFieldError::invalid_config("height_m must be positive"));
        }
        if self.cache_resolution < 2 {
            return Err(TerrainFieldError::invalid_config("cache_resolution must be at least 2"));
        }
        if self.height_variation_m <= 0.0 {
            return Err(TerrainFieldError::invalid_config("height_variation_m must be positive"));
        }
        if self.slope_probe_m <= 0.0 {
            return Err(TerrainFieldError::invalid_config("slope_probe_m must be positive"));
        }

        Ok(self)
    }
}

impl Default for TerrainFieldConfig {
    fn default() -> Self {
        Self {
            width_m: HERO_BIOME_WIDTH_METERS,
            height_m: HERO_BIOME_HEIGHT_METERS,
            cache_resolution: 129,
            height_base_m: 22.0,
            height_variation_m: 86.0,
            slope_probe_m: 6.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerrainScalarFieldSet {
    seed_hex: String,
    config: TerrainFieldConfig,
    samples: Vec<[f32; FIELD_COUNT]>,
}

impl TerrainScalarFieldSet {
    pub fn generate(seed: RootSeed, config: TerrainFieldConfig) -> Result<Self, TerrainFieldError> {
        let config = config.validate()?;
        let started = tracing::enabled!(Level::DEBUG).then(Instant::now);
        let sampler = TerrainSampler::new(seed, config);
        let resolution = usize::from(config.cache_resolution);
        let mut samples = Vec::with_capacity(resolution * resolution);

        for row in 0..resolution {
            let v = row as f32 / (resolution.saturating_sub(1) as f32);
            let y = config.height_m * v;
            for column in 0..resolution {
                let u = column as f32 / (resolution.saturating_sub(1) as f32);
                let x = config.width_m * u;
                let sample = sampler.sample_direct(Vec2::new(x, y));
                samples.push(sample.into_array());
            }
        }

        let seed_hex = seed.to_hex();
        if let Some(started) = started {
            debug!(
                seed_hex = %seed_hex,
                cache_resolution = config.cache_resolution,
                width_m = config.width_m,
                height_m = config.height_m,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                "generated terrain scalar field cache",
            );
        }

        Ok(Self { seed_hex, config, samples })
    }

    pub fn seed_hex(&self) -> &str {
        &self.seed_hex
    }

    pub fn config(&self) -> TerrainFieldConfig {
        self.config
    }

    pub fn sample(&self, point: Vec2) -> TerrainFieldSample {
        TerrainFieldSample::from_array(self.bilinear_sample(point))
    }

    pub fn sample_field(&self, field: TerrainFieldKind, point: Vec2) -> f32 {
        self.bilinear_sample(point)[field.as_index()]
    }

    pub fn summary_report(&self) -> TerrainFieldSummaryReport {
        let mut summaries = Vec::with_capacity(TerrainFieldKind::ALL.len());
        for field in TerrainFieldKind::ALL {
            let mut min = f32::INFINITY;
            let mut max = f32::NEG_INFINITY;
            let mut sum = 0.0;
            let mut sum_squares = 0.0;

            for sample in &self.samples {
                let value = sample[field.as_index()];
                min = min.min(value);
                max = max.max(value);
                sum += value;
                sum_squares += value * value;
            }

            let count = self.samples.len() as f32;
            let mean = sum / count;
            let variance = (sum_squares / count) - (mean * mean);
            summaries.push(TerrainFieldSummary {
                field,
                min,
                max,
                mean,
                stddev: variance.max(0.0).sqrt(),
            });
        }

        TerrainFieldSummaryReport {
            seed_hex: self.seed_hex.clone(),
            width_m: self.config.width_m,
            height_m: self.config.height_m,
            cache_resolution: self.config.cache_resolution,
            summaries,
        }
    }

    pub fn debug_dump(&self, resolution: u16) -> Result<TerrainFieldDebugDump, TerrainFieldError> {
        if resolution < 2 {
            return Err(TerrainFieldError::invalid_config(
                "debug dump resolution must be at least 2",
            ));
        }

        let mut fields = BTreeMap::new();
        for field in TerrainFieldKind::ALL {
            let mut quantized =
                Vec::with_capacity(usize::from(resolution) * usize::from(resolution));
            for row in 0..resolution {
                let v = row as f32 / (resolution.saturating_sub(1) as f32);
                let y = self.config.height_m * v;
                for column in 0..resolution {
                    let u = column as f32 / (resolution.saturating_sub(1) as f32);
                    let x = self.config.width_m * u;
                    let scalar = quantize01(self.sample_field_normalized(field, Vec2::new(x, y)));
                    quantized.push(scalar);
                }
            }
            fields.insert(field.as_str().to_owned(), quantized);
        }

        Ok(TerrainFieldDebugDump {
            seed_hex: self.seed_hex.clone(),
            resolution,
            summaries: self.summary_report().summaries,
            fields,
        })
    }

    pub fn render_overlay(
        &self,
        field: TerrainFieldKind,
        resolution: u16,
    ) -> Result<TerrainFieldOverlay, TerrainFieldError> {
        if resolution < 2 {
            return Err(TerrainFieldError::invalid_config("overlay resolution must be at least 2"));
        }

        let mut rgba8 = Vec::with_capacity(usize::from(resolution) * usize::from(resolution) * 4);
        for row in 0..resolution {
            let v = row as f32 / (resolution.saturating_sub(1) as f32);
            let y = self.config.height_m * v;
            for column in 0..resolution {
                let u = column as f32 / (resolution.saturating_sub(1) as f32);
                let x = self.config.width_m * u;
                let value = self.sample_field_normalized(field, Vec2::new(x, y));
                rgba8.extend(field_color(field, value));
            }
        }

        Ok(TerrainFieldOverlay { field, width: resolution, height: resolution, rgba8 })
    }

    fn sample_field_normalized(&self, field: TerrainFieldKind, point: Vec2) -> f32 {
        let value = self.sample_field(field, point);
        match field {
            TerrainFieldKind::Height => clamp01(inverse_lerp(
                self.config.height_base_m,
                self.config.height_base_m + self.config.height_variation_m,
                value,
            )),
            _ => clamp01(value),
        }
    }

    fn bilinear_sample(&self, point: Vec2) -> [f32; FIELD_COUNT] {
        let clamped =
            point.clamp(Vec2::new(0.0, 0.0), Vec2::new(self.config.width_m, self.config.height_m));
        let max_index = f32::from(self.config.cache_resolution.saturating_sub(1));
        let sample_x = if self.config.width_m == 0.0 {
            0.0
        } else {
            (clamped.x / self.config.width_m) * max_index
        };
        let sample_y = if self.config.height_m == 0.0 {
            0.0
        } else {
            (clamped.y / self.config.height_m) * max_index
        };

        let x0 = sample_x.floor() as usize;
        let y0 = sample_y.floor() as usize;
        let x1 = (x0 + 1).min(usize::from(self.config.cache_resolution) - 1);
        let y1 = (y0 + 1).min(usize::from(self.config.cache_resolution) - 1);
        let tx = sample_x - (x0 as f32);
        let ty = sample_y - (y0 as f32);
        let top_left = self.samples[self.index(x0, y0)];
        let top_right = self.samples[self.index(x1, y0)];
        let bottom_left = self.samples[self.index(x0, y1)];
        let bottom_right = self.samples[self.index(x1, y1)];

        let mut output = [0.0; FIELD_COUNT];
        for field_index in 0..FIELD_COUNT {
            let top = lerp(top_left[field_index], top_right[field_index], tx);
            let bottom = lerp(bottom_left[field_index], bottom_right[field_index], tx);
            output[field_index] = lerp(top, bottom, ty);
        }

        output
    }

    fn index(&self, column: usize, row: usize) -> usize {
        (row * usize::from(self.config.cache_resolution)) + column
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainFieldSample {
    pub height_m: f32,
    pub slope: f32,
    pub drainage: f32,
    pub moisture: f32,
    pub fog: f32,
    pub canopy_opportunity: f32,
    pub deadfall_probability: f32,
    pub hero_path_bias: f32,
}

impl TerrainFieldSample {
    fn into_array(self) -> [f32; FIELD_COUNT] {
        [
            self.height_m,
            self.slope,
            self.drainage,
            self.moisture,
            self.fog,
            self.canopy_opportunity,
            self.deadfall_probability,
            self.hero_path_bias,
        ]
    }

    fn from_array(values: [f32; FIELD_COUNT]) -> Self {
        Self {
            height_m: values[0],
            slope: values[1],
            drainage: values[2],
            moisture: values[3],
            fog: values[4],
            canopy_opportunity: values[5],
            deadfall_probability: values[6],
            hero_path_bias: values[7],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainFieldSummary {
    pub field: TerrainFieldKind,
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub stddev: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainFieldSummaryReport {
    pub seed_hex: String,
    pub width_m: f32,
    pub height_m: f32,
    pub cache_resolution: u16,
    pub summaries: Vec<TerrainFieldSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainFieldDebugDump {
    pub seed_hex: String,
    pub resolution: u16,
    pub summaries: Vec<TerrainFieldSummary>,
    pub fields: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainFieldOverlay {
    pub field: TerrainFieldKind,
    pub width: u16,
    pub height: u16,
    pub rgba8: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainFieldError {
    reason: String,
}

impl TerrainFieldError {
    fn invalid_config(reason: impl Into<String>) -> Self {
        Self { reason: reason.into() }
    }
}

impl std::fmt::Display for TerrainFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for TerrainFieldError {}

#[derive(Debug, Clone, Copy)]
struct TerrainSampler {
    config: TerrainFieldConfig,
    landform_noise: FractalNoise2,
    ridge_noise: FractalNoise2,
    moisture_noise: FractalNoise2,
    canopy_noise: FractalNoise2,
    deadfall_noise: FractalNoise2,
    path_wobble_noise: FractalNoise2,
}

impl TerrainSampler {
    fn new(seed: RootSeed, config: TerrainFieldConfig) -> Self {
        Self {
            config,
            landform_noise: FractalNoise2::new(
                seed.derive_stream_u64("terrain.landform"),
                0.0055,
                4,
                2.15,
                0.52,
            ),
            ridge_noise: FractalNoise2::new(
                seed.derive_stream_u64("terrain.ridges"),
                0.011,
                4,
                2.0,
                0.48,
            ),
            moisture_noise: FractalNoise2::new(
                seed.derive_stream_u64("terrain.moisture"),
                0.008,
                4,
                2.05,
                0.55,
            ),
            canopy_noise: FractalNoise2::new(
                seed.derive_stream_u64("terrain.canopy"),
                0.0095,
                3,
                2.0,
                0.5,
            ),
            deadfall_noise: FractalNoise2::new(
                seed.derive_stream_u64("terrain.deadfall"),
                0.017,
                3,
                2.2,
                0.45,
            ),
            path_wobble_noise: FractalNoise2::new(
                seed.derive_stream_u64("terrain.path"),
                0.0065,
                2,
                2.0,
                0.5,
            ),
        }
    }

    fn sample_direct(self, point: Vec2) -> TerrainFieldSample {
        let normalized = Vec2::new(point.x / self.config.width_m, point.y / self.config.height_m);
        let height_normalized = self.height_normalized_at(point);
        let height_m =
            self.config.height_base_m + (height_normalized * self.config.height_variation_m);

        let slope = self.slope(point);
        let path_center = 0.54
            + ((normalized.x - 0.5) * 0.14)
            + ((self.path_wobble_noise.sample01(point) - 0.5) * 0.14)
            + (((normalized.x * std::f32::consts::TAU * 1.1).sin()) * 0.04);
        let path_distance = (normalized.y - path_center).abs();
        let hero_path_bias = clamp01(1.0 - smootherstep01(path_distance / 0.16));

        let drainage = clamp01(
            ((1.0 - height_normalized) * 0.48)
                + ((1.0 - slope) * 0.16)
                + (self.moisture_noise.sample01(point) * 0.24)
                + ((1.0 - hero_path_bias) * 0.12),
        );
        let moisture = clamp01(
            (drainage * 0.46)
                + ((1.0 - height_normalized) * 0.22)
                + (self.moisture_noise.sample01(Vec2::new(point.x + 31.0, point.y - 17.0)) * 0.22)
                + ((1.0 - hero_path_bias) * 0.10),
        );
        let fog =
            clamp01((moisture * 0.62) + ((1.0 - height_normalized) * 0.28) + (drainage * 0.10));
        let canopy_opportunity = clamp01(
            ((1.0 - slope) * 0.34)
                + (moisture * 0.24)
                + ((1.0 - hero_path_bias) * 0.18)
                + (self.canopy_noise.sample01(point) * 0.24),
        );
        let deadfall_probability = clamp01(
            (slope * 0.28)
                + (drainage * 0.24)
                + ((1.0 - canopy_opportunity) * 0.20)
                + ((1.0 - hero_path_bias) * 0.12)
                + (self.deadfall_noise.sample01(point) * 0.16),
        );

        TerrainFieldSample {
            height_m,
            slope,
            drainage,
            moisture,
            fog,
            canopy_opportunity,
            deadfall_probability,
            hero_path_bias,
        }
    }

    fn slope(self, point: Vec2) -> f32 {
        let probe = self.config.slope_probe_m;
        let left_x = (point.x - probe).max(0.0);
        let right_x = (point.x + probe).min(self.config.width_m);
        let down_y = (point.y - probe).max(0.0);
        let up_y = (point.y + probe).min(self.config.height_m);
        let x0 = self.height_only(Vec2::new(left_x, point.y));
        let x1 = self.height_only(Vec2::new(right_x, point.y));
        let y0 = self.height_only(Vec2::new(point.x, down_y));
        let y1 = self.height_only(Vec2::new(point.x, up_y));
        let x_span = (right_x - left_x).max(f32::EPSILON);
        let y_span = (up_y - down_y).max(f32::EPSILON);
        let gradient = ((x1 - x0).abs() / x_span) + ((y1 - y0).abs() / y_span);
        clamp01(gradient / 2.8)
    }

    fn height_only(self, point: Vec2) -> f32 {
        let height_normalized = self.height_normalized_at(point);
        self.config.height_base_m + (height_normalized * self.config.height_variation_m)
    }

    fn height_normalized_at(self, point: Vec2) -> f32 {
        let normalized = Vec2::new(point.x / self.config.width_m, point.y / self.config.height_m);
        let landform = self.landform_noise.sample01(point);
        let ridged = 1.0 - ((self.ridge_noise.sample01(point) * 2.0) - 1.0).abs();
        let macro_rise = smootherstep01(normalized.y);
        let shoulder = smootherstep01(1.0 - (normalized.x - 0.5).abs() * 1.5);
        clamp01((landform * 0.58) + (ridged * 0.17) + (macro_rise * 0.15) + (shoulder * 0.10))
    }
}

fn quantize01(value: f32) -> u8 {
    (clamp01(value) * 255.0).round() as u8
}

fn field_color(field: TerrainFieldKind, value: f32) -> [u8; 4] {
    let t = clamp01(value);
    match field {
        TerrainFieldKind::Height => gradient(t, [27, 48, 36], [196, 151, 86]),
        TerrainFieldKind::Slope => gradient(t, [26, 32, 44], [240, 181, 104]),
        TerrainFieldKind::Drainage => gradient(t, [54, 41, 30], [75, 163, 118]),
        TerrainFieldKind::Moisture => gradient(t, [39, 28, 40], [93, 174, 173]),
        TerrainFieldKind::Fog => gradient(t, [32, 37, 48], [199, 214, 227]),
        TerrainFieldKind::CanopyOpportunity => gradient(t, [46, 29, 18], [116, 175, 95]),
        TerrainFieldKind::DeadfallProbability => gradient(t, [36, 32, 27], [173, 109, 68]),
        TerrainFieldKind::HeroPathBias => gradient(t, [28, 29, 35], [255, 214, 120]),
    }
}

fn gradient(value: f32, start: [u8; 3], end: [u8; 3]) -> [u8; 4] {
    [
        lerp(f32::from(start[0]), f32::from(end[0]), value).round() as u8,
        lerp(f32::from(start[1]), f32::from(end[1]), value).round() as u8,
        lerp(f32::from(start[2]), f32::from(end[2]), value).round() as u8,
        255,
    ]
}

#[cfg(test)]
mod tests {
    use insta::{assert_json_snapshot, assert_snapshot};
    use proptest::prelude::*;

    use super::*;

    fn canonical_config() -> TerrainFieldConfig {
        TerrainFieldConfig { cache_resolution: 65, ..TerrainFieldConfig::default() }
    }

    fn root_seed_from_u64(seed: u64) -> RootSeed {
        RootSeed::parse_hex(&format!("0x{seed:016X}")).expect("generated seed should parse")
    }

    #[test]
    fn summary_report_matches_hero_forest_seed() {
        let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let fields = TerrainScalarFieldSet::generate(seed, canonical_config())
            .expect("field set should build");

        assert_json_snapshot!("hero_forest_summary", fields.summary_report());
    }

    #[test]
    fn debug_dump_matches_duel_focus_seed() {
        let seed = RootSeed::parse_hex("0xC0FFEE01").expect("seed should parse");
        let fields = TerrainScalarFieldSet::generate(seed, canonical_config())
            .expect("field set should build");

        assert_json_snapshot!(
            "duel_focus_debug_dump",
            fields.debug_dump(4).expect("debug dump should build")
        );
    }

    #[test]
    fn overlay_pixels_match_requested_resolution() {
        let seed = RootSeed::parse_hex("0xF00DFACE").expect("seed should parse");
        let fields = TerrainScalarFieldSet::generate(seed, canonical_config())
            .expect("field set should build");
        let overlay =
            fields.render_overlay(TerrainFieldKind::Fog, 16).expect("overlay should render");

        assert_eq!(overlay.width, 16);
        assert_eq!(overlay.height, 16);
        assert_eq!(overlay.rgba8.len(), 16 * 16 * 4);
    }

    #[test]
    fn deterministic_generation_rebuilds_identical_cache() {
        let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let first = TerrainScalarFieldSet::generate(seed, canonical_config())
            .expect("field set should build");
        let second = TerrainScalarFieldSet::generate(seed, canonical_config())
            .expect("field set should rebuild");

        assert_eq!(first, second);
    }

    #[test]
    fn field_kind_names_stay_stable() {
        let names = TerrainFieldKind::ALL.iter().map(|field| field.as_str()).collect::<Vec<_>>();

        assert_snapshot!(
            names.join("\n"),
            @r#"
height
slope
drainage
moisture
fog
canopy_opportunity
deadfall_probability
hero_path_bias
"#
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(24))]

        #[test]
        fn generated_fields_stay_in_expected_bounds(seed in any::<u64>(), x in 0.0f32..HERO_BIOME_WIDTH_METERS, y in 0.0f32..HERO_BIOME_HEIGHT_METERS) {
            let fields = TerrainScalarFieldSet::generate(root_seed_from_u64(seed), canonical_config()).expect("field set should build");
            let sample = fields.sample(Vec2::new(x, y));

            prop_assert!((fields.config.height_base_m..=fields.config.height_base_m + fields.config.height_variation_m).contains(&sample.height_m));
            prop_assert!((0.0..=1.0).contains(&sample.slope));
            prop_assert!((0.0..=1.0).contains(&sample.drainage));
            prop_assert!((0.0..=1.0).contains(&sample.moisture));
            prop_assert!((0.0..=1.0).contains(&sample.fog));
            prop_assert!((0.0..=1.0).contains(&sample.canopy_opportunity));
            prop_assert!((0.0..=1.0).contains(&sample.deadfall_probability));
            prop_assert!((0.0..=1.0).contains(&sample.hero_path_bias));
        }

        #[test]
        fn nearby_samples_change_smoothly(seed in any::<u64>(), x in 0.0f32..(HERO_BIOME_WIDTH_METERS - 1.0), y in 0.0f32..(HERO_BIOME_HEIGHT_METERS - 1.0)) {
            let fields = TerrainScalarFieldSet::generate(root_seed_from_u64(seed), canonical_config()).expect("field set should build");
            let current = fields.sample(Vec2::new(x, y));
            let nearby = fields.sample(Vec2::new(x + 0.75, y + 0.75));

            prop_assert!((current.height_m - nearby.height_m).abs() < 6.5);
            prop_assert!((current.slope - nearby.slope).abs() < 0.18);
            prop_assert!((current.moisture - nearby.moisture).abs() < 0.14);
            prop_assert!((current.hero_path_bias - nearby.hero_path_bias).abs() < 0.16);
        }
    }
}
