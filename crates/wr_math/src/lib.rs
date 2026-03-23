#![forbid(unsafe_code)]

use wr_core::{CrateBoundary, CrateEntryPoint};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_math", CrateBoundary::Subsystem, false)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const fn splat(value: f32) -> Self {
        Self { x: value, y: value }
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self { x: self.x.clamp(min.x, max.x), y: self.y.clamp(min.y, max.y) }
    }
}

pub fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + ((end - start) * t)
}

/// Returns `0.0` for a degenerate range so callers can treat "no span" as the
/// minimum normalized value instead of propagating a NaN sentinel.
pub fn inverse_lerp(start: f32, end: f32, value: f32) -> f32 {
    let range = end - start;
    if range.abs() <= f32::EPSILON { 0.0 } else { (value - start) / range }
}

pub fn smootherstep01(value: f32) -> f32 {
    let t = clamp01(value);
    t * t * t * (t * ((t * 6.0) - 15.0) + 10.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FractalNoise2 {
    seed: u64,
    pub base_frequency: f32,
    pub octaves: u8,
    pub lacunarity: f32,
    pub gain: f32,
}

impl FractalNoise2 {
    pub fn new(seed: u64, base_frequency: f32, octaves: u8, lacunarity: f32, gain: f32) -> Self {
        Self { seed, base_frequency, octaves, lacunarity, gain }
    }

    pub fn sample01(self, point: Vec2) -> f32 {
        let mut amplitude = 1.0;
        let mut frequency = self.base_frequency;
        let mut weighted_sum = 0.0;
        let mut amplitude_sum = 0.0;

        for octave in 0..self.octaves.max(1) {
            let octave_seed = mix_seed(self.seed, u64::from(octave));
            let value = value_noise01(octave_seed, point, frequency);
            weighted_sum += value * amplitude;
            amplitude_sum += amplitude;
            amplitude *= self.gain;
            frequency *= self.lacunarity;
        }

        if amplitude_sum == 0.0 { 0.0 } else { clamp01(weighted_sum / amplitude_sum) }
    }
}

fn value_noise01(seed: u64, point: Vec2, frequency: f32) -> f32 {
    let scaled = Vec2::new(point.x * frequency, point.y * frequency);
    let base_x = scaled.x.floor() as i32;
    let base_y = scaled.y.floor() as i32;
    let fract_x = scaled.x - (base_x as f32);
    let fract_y = scaled.y - (base_y as f32);
    let smooth_x = smootherstep01(fract_x);
    let smooth_y = smootherstep01(fract_y);

    let v00 = lattice_value(seed, base_x, base_y);
    let v10 = lattice_value(seed, base_x + 1, base_y);
    let v01 = lattice_value(seed, base_x, base_y + 1);
    let v11 = lattice_value(seed, base_x + 1, base_y + 1);

    let row0 = lerp(v00, v10, smooth_x);
    let row1 = lerp(v01, v11, smooth_x);
    clamp01(lerp(row0, row1, smooth_y))
}

fn lattice_value(seed: u64, x: i32, y: i32) -> f32 {
    // Sign-extend negative coordinates before mixing so `-1` and `u32::MAX` do not alias.
    let hashed = mix_seed(
        mix_seed(seed, (x as i64) as u64),
        ((y as i64) as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93),
    );
    // Use the top 24 bits as a stable mantissa-sized bucket and normalize to [0, 1].
    ((hashed >> 40) as f32) / ((1_u64 << 24) - 1) as f32
}

fn mix_seed(seed: u64, value: u64) -> u64 {
    let mut state = seed ^ value.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    state ^= state >> 30;
    state = state.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    state ^= state >> 27;
    state = state.wrapping_mul(0x94D0_49BB_1331_11EB);
    state ^ (state >> 31)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn fractal_noise_is_deterministic_for_same_seed_and_point() {
        let noise = FractalNoise2::new(0xDEADBEEF, 0.014, 4, 2.0, 0.5);
        let point = Vec2::new(123.25, 456.75);

        assert_eq!(noise.sample01(point), noise.sample01(point));
    }

    proptest! {
        #[test]
        fn fractal_noise_stays_in_unit_interval(seed in any::<u64>(), x in -10_000.0f32..10_000.0f32, y in -10_000.0f32..10_000.0f32) {
            let noise = FractalNoise2::new(seed, 0.01, 5, 2.05, 0.55);
            let sample = noise.sample01(Vec2::new(x, y));

            prop_assert!((0.0..=1.0).contains(&sample));
        }

        #[test]
        fn nearby_noise_samples_remain_continuous(seed in any::<u64>(), x in -256.0f32..256.0f32, y in -256.0f32..256.0f32) {
            let noise = FractalNoise2::new(seed, 0.008, 4, 2.0, 0.5);
            let a = noise.sample01(Vec2::new(x, y));
            let b = noise.sample01(Vec2::new(x + 0.25, y + 0.25));

            prop_assert!((a - b).abs() < 0.2);
        }
    }
}
