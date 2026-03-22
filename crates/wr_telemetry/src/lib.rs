#![forbid(unsafe_code)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use wr_core::{CrateBoundary, CrateEntryPoint};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_telemetry", CrateBoundary::Subsystem, false)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlatformMetadata {
    pub os: String,
    pub family: String,
    pub arch: String,
}

impl PlatformMetadata {
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_owned(),
            family: std::env::consts::FAMILY.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunTimestamps {
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SeedInfo {
    pub label: String,
    pub value_hex: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
}

impl SeedInfo {
    pub fn new(label: impl Into<String>, value_hex: impl Into<String>) -> Self {
        Self { label: label.into(), value_hex: value_hex.into(), stream: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunMetadata {
    pub command_name: String,
    pub run_id: String,
    pub git_sha: String,
    pub cwd: String,
    pub platform: PlatformMetadata,
    pub timestamps: RunTimestamps,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

impl RunMetadata {
    pub fn new(
        command_name: impl Into<String>,
        run_id: impl Into<String>,
        git_sha: impl Into<String>,
        cwd: impl Into<String>,
        platform: PlatformMetadata,
        timestamps: RunTimestamps,
    ) -> Self {
        Self {
            command_name: command_name.into(),
            run_id: run_id.into(),
            git_sha: git_sha.into(),
            cwd: cwd.into(),
            platform,
            timestamps,
            notes: None,
        }
    }
}
