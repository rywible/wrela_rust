#![forbid(unsafe_code)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use wr_core::{CrateBoundary, CrateEntryPoint};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_render_api", CrateBoundary::Subsystem, false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RenderSize {
    pub width: u32,
    pub height: u32,
}

impl RenderSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn pixel_count(self) -> u64 {
        self.width as u64 * self.height as u64
    }

    pub fn validate(self) -> Result<Self, String> {
        if self.width == 0 || self.height == 0 {
            return Err(String::from("render target dimensions must be greater than zero"));
        }

        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ColorRgba8 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl ColorRgba8 {
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self { red, green, blue, alpha }
    }

    pub const fn normalized(self) -> [f64; 4] {
        [
            self.red as f64 / 255.0,
            self.green as f64 / 255.0,
            self.blue as f64 / 255.0,
            self.alpha as f64 / 255.0,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RenderColorSpace {
    Rgba8UnormSrgb,
}

impl RenderColorSpace {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rgba8UnormSrgb => "rgba8_unorm_srgb",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GraphicsAdapterInfo {
    pub backend: String,
    pub name: String,
    pub device_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver_info: Option<String>,
    pub shading_language: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OffscreenRenderRequest {
    pub size: RenderSize,
    pub clear_color: ColorRgba8,
}

impl OffscreenRenderRequest {
    pub fn validate(&self) -> Result<(), String> {
        self.size.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CapturedFrameInfo {
    pub size: RenderSize,
    pub color_space: RenderColorSpace,
    pub non_empty_pixels: u64,
}

pub fn clear_color_from_seed_hex(seed_hex: &str) -> ColorRgba8 {
    let trimmed = seed_hex.trim().trim_start_matches("0x").trim_start_matches("0X");

    let mut bytes = [0_u8; 4];
    for (index, chunk) in trimmed.as_bytes().rchunks(2).take(4).enumerate() {
        let value = std::str::from_utf8(chunk)
            .ok()
            .and_then(|chunk| u8::from_str_radix(chunk, 16).ok())
            .unwrap_or(0);
        bytes[3 - index] = value;
    }

    ColorRgba8 {
        red: bytes[0].saturating_add(32),
        green: bytes[1].saturating_add(48),
        blue: bytes[2].saturating_add(64),
        alpha: 255,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_derived_clear_colors_are_opaque_and_non_zero() {
        let color = clear_color_from_seed_hex("0x00000000DEADBEEF");

        assert_eq!(color.alpha, 255);
        assert!(u16::from(color.red) + u16::from(color.green) + u16::from(color.blue) > 0);
    }

    #[test]
    fn render_size_rejects_zero_dimensions() {
        let error = RenderSize::new(0, 360).validate().expect_err("zero width should fail");

        assert!(error.contains("greater than zero"));
    }
}
