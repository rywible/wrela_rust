use std::path::PathBuf;
use std::process::{Command, Output};

use tempfile::tempdir;
use wr_tools_harness::{FailureKind, HarnessStatus, ScenarioExecutionReport};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root should exist")
}

fn run_headless(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wr_headless"))
        .current_dir(workspace_root())
        .args(args)
        .output()
        .expect("wr_headless should launch")
}

fn load_report(output: &Output) -> ScenarioExecutionReport {
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8");
    let report_path = stdout.lines().last().expect("stdout should contain the report path");
    let report = std::fs::read_to_string(report_path).expect("terminal report should be readable");

    serde_json::from_str(&report).expect("terminal report should deserialize")
}

#[test]
fn startup_scenario_cli_writes_valid_terminal_report() {
    let temp = tempdir().expect("temporary output root should be created");
    let output = run_headless(&[
        "--scenario",
        "scenarios/smoke/startup.ron",
        "--run-id",
        "startup-smoke",
        "--output-root",
        temp.path().to_string_lossy().as_ref(),
    ]);

    assert!(
        output.status.success(),
        "wr_headless should succeed: stdout=`{}` stderr=`{}`",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report = load_report(&output);

    assert_eq!(report.result.status, HarnessStatus::Passed);
    assert_eq!(report.scenario_path, "scenarios/smoke/startup.ron");
    assert_eq!(report.metrics.frames_simulated, 16);
    assert_eq!(report.metrics.spawned_actor_count, 2);
    assert_eq!(report.assertions.len(), 2);
    assert!(
        report.artifacts.iter().any(|artifact| artifact
            .path
            .ends_with("reports/harness/run-scenario/startup-smoke/terminal_report.json")),
        "terminal report artifact should use the stable harness path contract"
    );
}

#[test]
fn same_scenario_and_seed_produce_identical_determinism_hashes() {
    let temp = tempdir().expect("temporary output root should be created");
    let first = run_headless(&[
        "--scenario",
        "scenarios/smoke/startup.ron",
        "--run-id",
        "determinism-a",
        "--output-root",
        temp.path().to_string_lossy().as_ref(),
    ]);
    let second = run_headless(&[
        "--scenario",
        "scenarios/smoke/startup.ron",
        "--run-id",
        "determinism-b",
        "--output-root",
        temp.path().to_string_lossy().as_ref(),
    ]);

    assert!(first.status.success(), "first run should succeed");
    assert!(second.status.success(), "second run should succeed");

    let first_report = load_report(&first);
    let second_report = load_report(&second);

    assert_eq!(first_report.determinism_hash, second_report.determinism_hash);
    assert_eq!(first_report.metrics, second_report.metrics);
    assert_eq!(first_report.assertions, second_report.assertions);
}

#[test]
fn failing_assertion_still_writes_terminal_report() {
    let temp = tempdir().expect("temporary output root should be created");
    let output = run_headless(&[
        "--scenario",
        "scenarios/smoke/assertion_failure.ron",
        "--run-id",
        "assertion-failure",
        "--output-root",
        temp.path().to_string_lossy().as_ref(),
    ]);

    assert!(
        !output.status.success(),
        "the failing scenario should exit non-zero so automation can classify it"
    );

    let report = load_report(&output);

    assert_eq!(report.result.status, HarnessStatus::Failed);
    assert_eq!(report.result.failure_kind, Some(FailureKind::ScenarioFailed));
    assert_eq!(report.metrics.frames_simulated, 1);
    assert!(
        report
            .result
            .details
            .as_deref()
            .is_some_and(|details| details.contains("world.actor_count")),
        "the failure detail should identify the failing metric"
    );
}
