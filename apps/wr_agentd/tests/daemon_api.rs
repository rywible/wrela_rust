use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tempfile::TempDir;
use tower::util::ServiceExt;
use wr_agentd::{AgentDaemonConfig, AgentDaemonState, CommandRunnerSpec, app};
use wr_tools_harness::{
    DaemonCommandRequest, DaemonJobSnapshot, DaemonJobState, DaemonLaunchRequest,
};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root should resolve")
}

fn fake_runner_script(temp_dir: &TempDir) -> PathBuf {
    let script_path = temp_dir.path().join("fake_xtask_runner.py");
    std::fs::write(
        &script_path,
        r#"#!/usr/bin/env python3
import json
import pathlib
import sys
import time

args = sys.argv[1:]
command = args[0]
run_id = "missing-run-id"
if "--run-id" in args:
    run_id = args[args.index("--run-id") + 1]

report_command = {
    "verify": "verify",
    "run-scenario": "run-scenario",
    "capture": "capture",
    "lookdev": "lookdev",
    "perf": "perf",
}[command]

report_path = pathlib.Path("reports/harness") / report_command / run_id / "terminal_report.json"
report_path.parent.mkdir(parents=True, exist_ok=True)

print(f"stdout:{command}:{run_id}", flush=True)
print(f"stderr:{command}:{run_id}", file=sys.stderr, flush=True)

if "slow" in run_id:
    time.sleep(0.35)

report = {
    "schema_version": "wr_harness/v1",
    "command_name": command,
    "run_id": run_id,
}
with open(report_path, "w", encoding="utf-8") as handle:
    json.dump(report, handle)
    handle.write("\n")

if "fail" in run_id:
    sys.exit(7)
"#,
    )
    .expect("fake runner script should write");
    script_path
}

fn fake_config(temp_dir: &TempDir) -> AgentDaemonConfig {
    let mut config = AgentDaemonConfig::local(temp_dir.path().to_path_buf());
    config.command_runner = CommandRunnerSpec {
        program: "python3".to_owned(),
        prefix_args: vec![fake_runner_script(temp_dir).to_string_lossy().into_owned()],
    };
    config
}

async fn post_job(router: &axum::Router, request: DaemonLaunchRequest) -> DaemonJobSnapshot {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).expect("request should serialize")))
                .expect("job launch request should build"),
        )
        .await
        .expect("job launch response should resolve");

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("launch response body should read");
    serde_json::from_slice(&body).expect("launch response should parse")
}

async fn get_job(router: &axum::Router, job_id: &str) -> DaemonJobSnapshot {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/jobs/{job_id}"))
                .body(Body::empty())
                .expect("job status request should build"),
        )
        .await
        .expect("job status response should resolve");

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("status response body should read");
    serde_json::from_slice(&body).expect("status response should parse")
}

async fn wait_for_completion(
    router: &axum::Router,
    job_id: &str,
    timeout: Duration,
) -> DaemonJobSnapshot {
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let snapshot = get_job(router, job_id).await;
        if matches!(snapshot.state, DaemonJobState::Succeeded | DaemonJobState::Failed) {
            return snapshot;
        }
        assert!(tokio::time::Instant::now() < deadline, "job `{job_id}` timed out");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

fn artifact_path(snapshot: &DaemonJobSnapshot, role: &str) -> String {
    snapshot
        .artifacts
        .iter()
        .find(|artifact| artifact.role == role)
        .map(|artifact| artifact.path.clone())
        .unwrap_or_else(|| panic!("missing `{role}` artifact"))
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let temp_dir = TempDir::new().expect("temp dir should exist");
    let router = app(AgentDaemonState::new(fake_config(&temp_dir)));

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/healthz")
                .body(Body::empty())
                .expect("health request should build"),
        )
        .await
        .expect("health response should resolve");

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("health body should read");
    let payload: Value = serde_json::from_slice(&body).expect("health body should parse");

    assert_eq!(payload["schema_version"], "wr_harness/v1");
    assert_eq!(payload["status"], "ok");
}

#[tokio::test]
async fn launch_endpoint_accepts_each_public_command_surface() {
    let temp_dir = TempDir::new().expect("temp dir should exist");
    let router = app(AgentDaemonState::new(fake_config(&temp_dir)));
    let requests = [
        DaemonLaunchRequest {
            command: DaemonCommandRequest::Verify { run_id: Some("verify-contract".to_owned()) },
        },
        DaemonLaunchRequest {
            command: DaemonCommandRequest::RunScenario {
                scenario_path: "scenarios/smoke/startup.ron".to_owned(),
                run_id: Some("scenario-contract".to_owned()),
            },
        },
        DaemonLaunchRequest {
            command: DaemonCommandRequest::CaptureFrames {
                scenario_path: "scenarios/traversal/hero_path.ron".to_owned(),
                run_id: Some("capture-contract".to_owned()),
            },
        },
        DaemonLaunchRequest {
            command: DaemonCommandRequest::LookdevSweep {
                tweak_pack_path: "tweak_packs/release/hero_forest.ron".to_owned(),
                camera_set: "forest_hero".to_owned(),
                seed_hex: "0xDEADBEEF".to_owned(),
                run_id: Some("lookdev-contract".to_owned()),
            },
        },
        DaemonLaunchRequest {
            command: DaemonCommandRequest::PerfCheck {
                scenario_path: "scenarios/traversal/hero_path.ron".to_owned(),
                run_id: Some("perf-contract".to_owned()),
            },
        },
    ];

    for request in requests {
        let launched = post_job(&router, request).await;
        let terminal_report = artifact_path(&launched, "terminal_report");

        assert_eq!(launched.schema_version, "wr_harness/v1");
        assert!(terminal_report.ends_with("terminal_report.json"));

        let completed =
            wait_for_completion(&router, &launched.job_id, Duration::from_secs(2)).await;
        assert_eq!(completed.state, DaemonJobState::Succeeded);
        assert!(
            temp_dir.path().join(&terminal_report).exists(),
            "fake runner should write the predicted terminal report"
        );
    }
}

#[tokio::test]
async fn subprocess_supervision_captures_logs_and_exit_codes() {
    let temp_dir = TempDir::new().expect("temp dir should exist");
    let router = app(AgentDaemonState::new(fake_config(&temp_dir)));
    let launched = post_job(
        &router,
        DaemonLaunchRequest {
            command: DaemonCommandRequest::Verify { run_id: Some("verify-fail".to_owned()) },
        },
    )
    .await;

    let completed = wait_for_completion(&router, &launched.job_id, Duration::from_secs(2)).await;
    let stdout_path = temp_dir.path().join(artifact_path(&completed, "daemon_stdout"));
    let stderr_path = temp_dir.path().join(artifact_path(&completed, "daemon_stderr"));

    assert_eq!(completed.state, DaemonJobState::Failed);
    assert_eq!(completed.exit_code, Some(7));
    assert!(
        std::fs::read_to_string(stdout_path)
            .expect("stdout log should exist")
            .contains("stdout:verify:verify-fail")
    );
    assert!(
        std::fs::read_to_string(stderr_path)
            .expect("stderr log should exist")
            .contains("stderr:verify:verify-fail")
    );
}

#[tokio::test]
async fn concurrent_jobs_preserve_distinct_output_directories() {
    let temp_dir = TempDir::new().expect("temp dir should exist");
    let router = app(AgentDaemonState::new(fake_config(&temp_dir)));
    let first = post_job(
        &router,
        DaemonLaunchRequest {
            command: DaemonCommandRequest::RunScenario {
                scenario_path: "scenarios/smoke/startup.ron".to_owned(),
                run_id: Some("slow-alpha".to_owned()),
            },
        },
    )
    .await;
    let second = post_job(
        &router,
        DaemonLaunchRequest {
            command: DaemonCommandRequest::RunScenario {
                scenario_path: "scenarios/smoke/startup.ron".to_owned(),
                run_id: Some("slow-beta".to_owned()),
            },
        },
    )
    .await;

    tokio::time::sleep(Duration::from_millis(75)).await;
    let first_mid = get_job(&router, &first.job_id).await;
    let second_mid = get_job(&router, &second.job_id).await;

    assert_eq!(first_mid.state, DaemonJobState::Running);
    assert_eq!(second_mid.state, DaemonJobState::Running);
    assert_ne!(
        artifact_path(&first_mid, "daemon_run_directory"),
        artifact_path(&second_mid, "daemon_run_directory")
    );

    let first_done = wait_for_completion(&router, &first.job_id, Duration::from_secs(2)).await;
    let second_done = wait_for_completion(&router, &second.job_id, Duration::from_secs(2)).await;

    assert_eq!(first_done.state, DaemonJobState::Succeeded);
    assert_eq!(second_done.state, DaemonJobState::Succeeded);
}

#[tokio::test]
async fn daemon_run_scenario_matches_cli_payload_on_real_xtask_path() {
    let router = app(AgentDaemonState::new(AgentDaemonConfig::local(workspace_root())));
    let daemon_run_id = "daemon-real-parity";
    let cli_run_id = "cli-real-parity";

    let launched = post_job(
        &router,
        DaemonLaunchRequest {
            command: DaemonCommandRequest::RunScenario {
                scenario_path: "scenarios/smoke/startup.ron".to_owned(),
                run_id: Some(daemon_run_id.to_owned()),
            },
        },
    )
    .await;

    let completed = wait_for_completion(&router, &launched.job_id, Duration::from_secs(20)).await;
    assert_eq!(completed.state, DaemonJobState::Succeeded);

    let cli_output = Command::new("cargo")
        .args(["xtask", "run-scenario", "scenarios/smoke/startup.ron", "--run-id", cli_run_id])
        .current_dir(workspace_root())
        .output()
        .expect("direct xtask run should complete");
    assert!(
        cli_output.status.success(),
        "direct xtask run should succeed: {}",
        String::from_utf8_lossy(&cli_output.stderr)
    );

    let daemon_report_path = workspace_root()
        .join(format!("reports/harness/run-scenario/{daemon_run_id}/terminal_report.json"));
    let cli_report_path = workspace_root()
        .join(format!("reports/harness/run-scenario/{cli_run_id}/terminal_report.json"));
    let daemon_report: Value = serde_json::from_slice(
        &std::fs::read(&daemon_report_path).expect("daemon terminal report should exist"),
    )
    .expect("daemon report should parse");
    let cli_report: Value =
        serde_json::from_slice(&std::fs::read(&cli_report_path).expect("cli report should exist"))
            .expect("cli report should parse");

    assert_eq!(normalize_scenario_report(daemon_report), normalize_scenario_report(cli_report));
}

fn normalize_scenario_report(mut report: Value) -> Value {
    report.as_object_mut().expect("report should be an object").remove("metadata");
    report.as_object_mut().expect("report should be an object").remove("artifacts");
    if let Some(telemetry_summary) = report
        .get_mut("metrics")
        .and_then(Value::as_object_mut)
        .and_then(|metrics| metrics.get_mut("telemetry_summary"))
        .and_then(Value::as_object_mut)
    {
        telemetry_summary.remove("frame_samples");
        telemetry_summary.remove("frame_time_ms");
        telemetry_summary.remove("sim_time_ms");
    }
    report
}
