#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use tokio::fs::{self, File};
use tokio::io::copy;
use tokio::net::TcpListener;
use tokio::process::Command;
use tracing::{info, warn};
use wr_core::{CrateBoundary, CrateEntryPoint};
use wr_tools_harness::{
    ArtifactDescriptor, ArtifactLayout, DaemonCommandRequest, DaemonJobSnapshot, DaemonJobState,
    DaemonLaunchRequest,
};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_agentd", CrateBoundary::AppShell, true)
}

pub const fn target_runtime() -> CrateEntryPoint {
    wr_game::init_entrypoint()
}

const DAEMON_COMMAND_NAME: &str = "daemon";
const STDOUT_LOG_FILENAME: &str = "stdout.log";
const STDERR_LOG_FILENAME: &str = "stderr.log";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRunnerSpec {
    pub program: String,
    pub prefix_args: Vec<String>,
}

impl Default for CommandRunnerSpec {
    fn default() -> Self {
        Self { program: "cargo".to_owned(), prefix_args: vec!["xtask".to_owned()] }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDaemonConfig {
    pub bind_address: SocketAddr,
    pub workspace_root: PathBuf,
    pub command_runner: CommandRunnerSpec,
}

impl AgentDaemonConfig {
    pub fn local(workspace_root: PathBuf) -> Self {
        Self {
            bind_address: SocketAddr::from((Ipv4Addr::LOCALHOST, 8787)),
            workspace_root,
            command_runner: CommandRunnerSpec::default(),
        }
    }
}

#[derive(Clone)]
pub struct AgentDaemonState {
    inner: Arc<AgentDaemonStateInner>,
}

struct AgentDaemonStateInner {
    config: AgentDaemonConfig,
    jobs: Mutex<BTreeMap<String, DaemonJobSnapshot>>,
    next_job_number: AtomicU64,
}

impl AgentDaemonState {
    pub fn new(config: AgentDaemonConfig) -> Self {
        Self {
            inner: Arc::new(AgentDaemonStateInner {
                config,
                jobs: Mutex::new(BTreeMap::new()),
                next_job_number: AtomicU64::new(1),
            }),
        }
    }

    pub fn workspace_root(&self) -> PathBuf {
        self.inner.config.workspace_root.clone()
    }

    fn command_runner(&self) -> CommandRunnerSpec {
        self.inner.config.command_runner.clone()
    }

    fn allocate_job_id(&self) -> String {
        let next = self.inner.next_job_number.fetch_add(1, Ordering::Relaxed);
        format!("job-{next:04}")
    }

    fn insert_snapshot(&self, snapshot: DaemonJobSnapshot) {
        self.inner
            .jobs
            .lock()
            .expect("agent daemon jobs mutex should not be poisoned")
            .insert(snapshot.job_id.clone(), snapshot);
    }

    fn get_snapshot(&self, job_id: &str) -> Option<DaemonJobSnapshot> {
        self.inner
            .jobs
            .lock()
            .expect("agent daemon jobs mutex should not be poisoned")
            .get(job_id)
            .cloned()
    }

    fn update_snapshot(&self, job_id: &str, apply: impl FnOnce(&mut DaemonJobSnapshot)) {
        if let Some(snapshot) = self
            .inner
            .jobs
            .lock()
            .expect("agent daemon jobs mutex should not be poisoned")
            .get_mut(job_id)
        {
            apply(snapshot);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DaemonOptions {
    bind_address: SocketAddr,
    workspace_root: PathBuf,
}

impl DaemonOptions {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let cwd = std::env::current_dir()
            .map_err(|error| format!("failed to read current dir: {error}"))?;
        let mut options = Self {
            bind_address: SocketAddr::from((Ipv4Addr::LOCALHOST, 8787)),
            workspace_root: cwd,
        };

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bind" => {
                    let value =
                        args.next().ok_or_else(|| String::from("expected a value after --bind"))?;
                    options.bind_address = value
                        .parse()
                        .map_err(|error| format!("invalid --bind address `{value}`: {error}"))?;
                }
                "--workspace-root" => {
                    let value = args
                        .next()
                        .ok_or_else(|| String::from("expected a value after --workspace-root"))?;
                    options.workspace_root = PathBuf::from(value);
                }
                other => {
                    return Err(format!(
                        "unsupported argument `{other}` for daemon; supported flags: --bind, --workspace-root"
                    ));
                }
            }
        }

        Ok(options)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct HealthResponse {
    schema_version: String,
    status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ApiError {
    schema_version: String,
    error: String,
}

type ApiResult<T> = Result<(StatusCode, Json<T>), (StatusCode, Json<ApiError>)>;

pub fn run(mut args: impl Iterator<Item = String>) -> i32 {
    match run_daemon_command(&mut args) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("agent daemon failed: {error}");
            1
        }
    }
}

pub fn run_daemon_command(args: impl Iterator<Item = String>) -> Result<(), String> {
    let options = DaemonOptions::parse(args)?;
    let workspace_root = options.workspace_root.canonicalize().map_err(|error| {
        format!("failed to resolve workspace root `{}`: {error}", options.workspace_root.display())
    })?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("failed to create tokio runtime: {error}"))?;

    runtime.block_on(async move {
        let mut config = AgentDaemonConfig::local(workspace_root);
        config.bind_address = options.bind_address;
        serve_until_shutdown(config).await
    })
}

pub fn app(state: AgentDaemonState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/jobs", post(launch_job))
        .route("/v1/jobs/{job_id}", get(get_job))
        .with_state(state)
}

async fn serve_until_shutdown(config: AgentDaemonConfig) -> Result<(), String> {
    let listener = TcpListener::bind(config.bind_address)
        .await
        .map_err(|error| format!("failed to bind daemon listener: {error}"))?;
    let local_address = listener
        .local_addr()
        .map_err(|error| format!("failed to read bound daemon address: {error}"))?;
    let state = AgentDaemonState::new(config);

    println!("{local_address}");
    info!(bind = %local_address, "agent daemon listening");

    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|error| format!("agent daemon serve loop failed: {error}"))
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        warn!(error = %error, "failed to install ctrl-c handler for agent daemon");
    }
}

async fn healthz() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            schema_version: wr_tools_harness::HARNESS_SCHEMA_VERSION.to_owned(),
            status: "ok".to_owned(),
        }),
    )
}

async fn launch_job(
    State(state): State<AgentDaemonState>,
    Json(request): Json<DaemonLaunchRequest>,
) -> ApiResult<DaemonJobSnapshot> {
    if let Err(error) = request.validate() {
        return Err(api_error(StatusCode::BAD_REQUEST, error.to_string()));
    }

    let job_id = state.allocate_job_id();
    let run_id = request.command.run_id().map(ToOwned::to_owned).unwrap_or_else(|| {
        format!("{}-{job_id}", request.command.artifact_command_name().replace('-', "_"))
    });
    let command = request.command.with_run_id_if_missing(run_id);
    let daemon_layout = ArtifactLayout::new(DAEMON_COMMAND_NAME, &job_id);
    let daemon_run_directory = state.workspace_root().join(daemon_layout.run_directory());
    fs::create_dir_all(&daemon_run_directory).await.map_err(|error| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "failed to create daemon run directory `{}`: {error}",
                daemon_run_directory.display()
            ),
        )
    })?;

    let stdout_path = daemon_run_directory.join(STDOUT_LOG_FILENAME);
    let stderr_path = daemon_run_directory.join(STDERR_LOG_FILENAME);
    let snapshot = DaemonJobSnapshot {
        schema_version: wr_tools_harness::HARNESS_SCHEMA_VERSION.to_owned(),
        job_id: job_id.clone(),
        command: command.clone(),
        state: DaemonJobState::Queued,
        artifacts: daemon_job_artifacts(&job_id, &command, &stdout_path, &stderr_path),
        exit_code: None,
        started_at_unix_ms: None,
        completed_at_unix_ms: None,
        error: None,
    };
    state.insert_snapshot(snapshot.clone());

    tokio::spawn(run_job(state.clone(), job_id, command, stdout_path, stderr_path));

    Ok((StatusCode::ACCEPTED, Json(snapshot)))
}

async fn get_job(
    State(state): State<AgentDaemonState>,
    Path(job_id): Path<String>,
) -> ApiResult<DaemonJobSnapshot> {
    let snapshot = state
        .get_snapshot(&job_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, format!("job `{job_id}` was not found")))?;
    Ok((StatusCode::OK, Json(snapshot)))
}

async fn run_job(
    state: AgentDaemonState,
    job_id: String,
    command: DaemonCommandRequest,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
) {
    state.update_snapshot(&job_id, |snapshot| {
        snapshot.state = DaemonJobState::Running;
        snapshot.started_at_unix_ms = Some(now_unix_ms());
    });

    match execute_job_command(
        &state.workspace_root(),
        &state.command_runner(),
        &command,
        &stdout_path,
        &stderr_path,
    )
    .await
    {
        Ok(exit_code) => {
            state.update_snapshot(&job_id, |snapshot| {
                snapshot.state = if exit_code == Some(0) {
                    DaemonJobState::Succeeded
                } else {
                    DaemonJobState::Failed
                };
                snapshot.completed_at_unix_ms = Some(now_unix_ms());
                snapshot.exit_code = exit_code;
                if exit_code != Some(0) {
                    snapshot.error = Some(format!(
                        "subprocess exited with status {}",
                        exit_code
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| String::from("<unknown>"))
                    ));
                }
            });
        }
        Err(error) => {
            warn!(job_id = %job_id, error = %error, "agent daemon job execution failed");
            state.update_snapshot(&job_id, |snapshot| {
                snapshot.state = DaemonJobState::Failed;
                snapshot.completed_at_unix_ms = Some(now_unix_ms());
                snapshot.error = Some(error);
            });
        }
    }
}

async fn execute_job_command(
    workspace_root: &PathBuf,
    runner: &CommandRunnerSpec,
    command: &DaemonCommandRequest,
    stdout_path: &PathBuf,
    stderr_path: &PathBuf,
) -> Result<Option<i32>, String> {
    let mut child = Command::new(&runner.program);
    child.current_dir(workspace_root);
    child.args(&runner.prefix_args);
    child.args(command.xtask_args());
    child.env("CARGO_TERM_COLOR", "never");
    child.stdout(Stdio::piped());
    child.stderr(Stdio::piped());

    let mut child = child.spawn().map_err(|error| {
        format!("failed to spawn `{}` for `{}`: {error}", runner.program, command.command_label())
    })?;
    let stdout =
        child.stdout.take().ok_or_else(|| String::from("child process did not expose stdout"))?;
    let stderr =
        child.stderr.take().ok_or_else(|| String::from("child process did not expose stderr"))?;

    let stdout_file = File::create(stdout_path).await.map_err(|error| {
        format!("failed to create stdout log `{}`: {error}", stdout_path.display())
    })?;
    let stderr_file = File::create(stderr_path).await.map_err(|error| {
        format!("failed to create stderr log `{}`: {error}", stderr_path.display())
    })?;

    let stdout_task = tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut writer = stdout_file;
        copy(&mut reader, &mut writer)
            .await
            .map_err(|error| format!("failed to stream stdout to artifact: {error}"))
    });
    let stderr_task = tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stderr);
        let mut writer = stderr_file;
        copy(&mut reader, &mut writer)
            .await
            .map_err(|error| format!("failed to stream stderr to artifact: {error}"))
    });

    let status = child
        .wait()
        .await
        .map_err(|error| format!("failed to await child process completion: {error}"))?;
    let exit_code = status.code();

    stdout_task.await.map_err(|error| format!("stdout task join failed: {error}"))??;
    stderr_task.await.map_err(|error| format!("stderr task join failed: {error}"))??;

    Ok(exit_code)
}

fn daemon_job_artifacts(
    job_id: &str,
    command: &DaemonCommandRequest,
    _stdout_path: &PathBuf,
    _stderr_path: &PathBuf,
) -> Vec<ArtifactDescriptor> {
    let layout = ArtifactLayout::new(DAEMON_COMMAND_NAME, job_id);
    let mut artifacts = vec![
        ArtifactDescriptor {
            role: "daemon_stdout".to_owned(),
            path: layout.run_directory().join(STDOUT_LOG_FILENAME).to_string_lossy().into_owned(),
            media_type: "text/plain".to_owned(),
        },
        ArtifactDescriptor {
            role: "daemon_stderr".to_owned(),
            path: layout.run_directory().join(STDERR_LOG_FILENAME).to_string_lossy().into_owned(),
            media_type: "text/plain".to_owned(),
        },
    ];

    if let Some(run_id) = command.run_id() {
        artifacts.push(ArtifactDescriptor {
            role: "terminal_report".to_owned(),
            path: ArtifactLayout::new(command.artifact_command_name(), run_id)
                .terminal_report_path_string(),
            media_type: "application/json".to_owned(),
        });
    }

    artifacts.push(ArtifactDescriptor {
        role: "daemon_run_directory".to_owned(),
        path: layout.run_directory().to_string_lossy().into_owned(),
        media_type: "inode/directory".to_owned(),
    });
    artifacts
}

fn api_error(status: StatusCode, error: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            schema_version: wr_tools_harness::HARNESS_SCHEMA_VERSION.to_owned(),
            error: error.into(),
        }),
    )
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
