use std::fs::{self, File};
use std::path::{Path, PathBuf};

use tracing::dispatcher::DefaultGuard;
use tracing_subscriber::fmt;
use wr_core::{ProfilerBackend, TelemetryConfig};

#[derive(Debug)]
pub enum TraceCaptureError {
    Io(std::io::Error),
    ProfilerUnavailable(&'static str),
}

impl std::fmt::Display for TraceCaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "i/o error: {error}"),
            Self::ProfilerUnavailable(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for TraceCaptureError {}

impl From<std::io::Error> for TraceCaptureError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub struct TraceCapture {
    path: PathBuf,
    _guard: DefaultGuard,
}

impl TraceCapture {
    pub fn install(
        path: impl AsRef<Path>,
        config: &TelemetryConfig,
    ) -> Result<Option<Self>, TraceCaptureError> {
        if !config.enable_tracing {
            return Ok(None);
        }

        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let trace_file = File::create(&path)?;
        let subscriber = fmt()
            .json()
            .with_ansi(false)
            .with_current_span(true)
            .with_span_list(true)
            .flatten_event(true)
            .with_writer(move || {
                trace_file
                    .try_clone()
                    .expect("trace capture file should remain cloneable for the subscriber")
            })
            .finish();
        let guard = tracing::subscriber::set_default(subscriber);

        Ok(Some(Self { path, _guard: guard }))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub struct ProfilerSession {
    active_backend: ProfilerBackend,
    #[cfg(feature = "tracy")]
    _client: Option<tracy_client::Client>,
}

impl std::fmt::Debug for ProfilerSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfilerSession").field("active_backend", &self.active_backend).finish()
    }
}

impl ProfilerSession {
    pub fn start(config: &TelemetryConfig) -> Result<Self, TraceCaptureError> {
        match config.profiler_backend {
            ProfilerBackend::Disabled => Ok(Self {
                active_backend: ProfilerBackend::Disabled,
                #[cfg(feature = "tracy")]
                _client: None,
            }),
            ProfilerBackend::Tracy => start_tracy_session(),
        }
    }

    pub fn active_backend(&self) -> ProfilerBackend {
        self.active_backend
    }
}

#[cfg(feature = "tracy")]
fn start_tracy_session() -> Result<ProfilerSession, TraceCaptureError> {
    Ok(ProfilerSession {
        active_backend: ProfilerBackend::Tracy,
        _client: Some(tracy_client::Client::start()),
    })
}

#[cfg(not(feature = "tracy"))]
fn start_tracy_session() -> Result<ProfilerSession, TraceCaptureError> {
    Err(TraceCaptureError::ProfilerUnavailable(
        "tracy profiling requested but wr_telemetry was built without the `tracy` feature",
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;
    use wr_core::{ProfilerBackend, TelemetryConfig};

    use super::*;

    #[test]
    fn trace_capture_respects_disabled_tracing() {
        let temp = tempdir().expect("temporary directory should exist");
        let path = temp.path().join("trace.jsonl");

        let capture = TraceCapture::install(
            &path,
            &TelemetryConfig {
                enable_metrics: true,
                enable_tracing: false,
                profiler_backend: ProfilerBackend::Disabled,
            },
        )
        .expect("trace capture setup should succeed");

        assert!(capture.is_none());
        assert!(!path.exists());
    }

    #[test]
    fn trace_capture_writes_jsonl_events() {
        let temp = tempdir().expect("temporary directory should exist");
        let path = temp.path().join("trace.jsonl");

        let capture = TraceCapture::install(
            &path,
            &TelemetryConfig {
                enable_metrics: true,
                enable_tracing: true,
                profiler_backend: ProfilerBackend::Disabled,
            },
        )
        .expect("trace capture should succeed");

        tracing::info!(system = "telemetry_test", "trace smoke");
        drop(capture);

        let contents = fs::read_to_string(&path).expect("trace log should be readable");
        assert!(contents.contains("\"system\":\"telemetry_test\""));
        assert!(contents.contains("\"message\":\"trace smoke\""));
    }

    #[cfg(feature = "tracy")]
    #[test]
    fn tracy_feature_builds_profiler_session() {
        let session = ProfilerSession::start(&TelemetryConfig {
            enable_metrics: true,
            enable_tracing: true,
            profiler_backend: ProfilerBackend::Tracy,
        })
        .expect("tracy session should start when the feature is enabled");

        assert_eq!(session.active_backend(), ProfilerBackend::Tracy);
    }

    #[cfg(not(feature = "tracy"))]
    #[test]
    fn tracy_requests_fail_cleanly_without_feature() {
        let error = ProfilerSession::start(&TelemetryConfig {
            enable_metrics: true,
            enable_tracing: true,
            profiler_backend: ProfilerBackend::Tracy,
        })
        .expect_err("missing tracy feature should be reported");

        assert!(error.to_string().contains("built without the `tracy` feature"));
    }
}
