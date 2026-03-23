use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const ENV_ENABLE_METRICS: &str = "WRELA_ENABLE_METRICS";
const ENV_ENABLE_TRACING: &str = "WRELA_ENABLE_TRACING";
const ENV_PROFILER: &str = "WRELA_PROFILER";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelemetryConfigError {
    InvalidBoolean { env: &'static str, value: String },
    InvalidProfilerBackend { value: String },
}

impl std::fmt::Display for TelemetryConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBoolean { env, value } => {
                write!(f, "environment variable `{env}` expected a boolean value, found `{value}`")
            }
            Self::InvalidProfilerBackend { value } => {
                write!(
                    f,
                    "environment variable `{ENV_PROFILER}` expected `disabled` or `tracy`, found `{value}`"
                )
            }
        }
    }
}

impl std::error::Error for TelemetryConfigError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProfilerBackend {
    #[default]
    Disabled,
    Tracy,
}

impl ProfilerBackend {
    fn parse_env(value: &str) -> Result<Self, TelemetryConfigError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "disabled" | "off" | "none" => Ok(Self::Disabled),
            "tracy" => Ok(Self::Tracy),
            _ => Err(TelemetryConfigError::InvalidProfilerBackend { value: value.to_owned() }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TelemetryConfig {
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub profiler_backend: ProfilerBackend,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            enable_tracing: cfg!(debug_assertions),
            profiler_backend: ProfilerBackend::Disabled,
        }
    }
}

impl TelemetryConfig {
    pub fn headless_default() -> Self {
        Self {
            enable_metrics: true,
            enable_tracing: true,
            profiler_backend: ProfilerBackend::Disabled,
        }
    }

    pub fn from_env() -> Result<Self, TelemetryConfigError> {
        let mut config = Self::default();
        config.apply_env_overrides()?;
        Ok(config)
    }

    pub fn headless_from_env() -> Result<Self, TelemetryConfigError> {
        let mut config = Self::headless_default();
        config.apply_env_overrides()?;
        Ok(config)
    }

    fn apply_env_overrides(&mut self) -> Result<(), TelemetryConfigError> {
        if let Some(value) = std::env::var_os(ENV_ENABLE_METRICS) {
            self.enable_metrics =
                parse_bool_env(ENV_ENABLE_METRICS, &value.into_string().unwrap_or_default())?;
        }

        if let Some(value) = std::env::var_os(ENV_ENABLE_TRACING) {
            self.enable_tracing =
                parse_bool_env(ENV_ENABLE_TRACING, &value.into_string().unwrap_or_default())?;
        }

        if let Some(value) = std::env::var_os(ENV_PROFILER) {
            self.profiler_backend =
                ProfilerBackend::parse_env(&value.into_string().unwrap_or_default())?;
        }

        Ok(())
    }
}

fn parse_bool_env(env: &'static str, value: &str) -> Result<bool, TelemetryConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(TelemetryConfigError::InvalidBoolean { env, value: value.to_owned() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_default_keeps_metrics_and_tracing_enabled() {
        let config = TelemetryConfig::headless_default();

        assert!(config.enable_metrics);
        assert!(config.enable_tracing);
        assert_eq!(config.profiler_backend, ProfilerBackend::Disabled);
    }

    #[test]
    fn profiler_backend_parser_accepts_expected_values() {
        assert_eq!(
            ProfilerBackend::parse_env("disabled").expect("disabled should parse"),
            ProfilerBackend::Disabled
        );
        assert_eq!(
            ProfilerBackend::parse_env("tracy").expect("tracy should parse"),
            ProfilerBackend::Tracy
        );
    }

    #[test]
    fn profiler_backend_parser_rejects_unknown_values() {
        let error = ProfilerBackend::parse_env("capture").expect_err("invalid backend should fail");

        assert!(error.to_string().contains("disabled"));
    }
}
