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
    assert_eq!(report.seed.value_hex, "0x00000000DEADBEEF");
    assert_eq!(report.seed.derivations.len(), 7);
    assert_eq!(report.seed.config_pack.as_ref().map(|pack| pack.name.as_str()), Some("default"));
    assert!(
        report.seed.derivations.iter().any(|derivation| derivation.path == "combat.scenarios"
            && derivation.parent_path.as_deref() == Some("combat")),
        "scenario reports should include the stable sub-seed derivation tree"
    );
    assert_eq!(report.metrics.frames_simulated, 16);
    assert_eq!(report.metrics.spawned_actor_count, 2);
    assert_eq!(report.assertions.len(), 3);
    assert_eq!(
        report.metrics.telemetry_summary.as_ref().map(|summary| summary.frame_count),
        Some(16)
    );
    assert!(
        report.metrics.telemetry_summary.as_ref().is_some_and(|summary| {
            summary.tracing_enabled
                && summary.metrics_enabled
                && summary.frame_samples.len() == 16
                && summary.memory_bytes.max >= summary.memory_bytes.min
        }),
        "scenario telemetry should capture per-frame samples and aggregate memory counters"
    );
    assert!(
        report.artifacts.iter().any(|artifact| artifact
            .path
            .ends_with("reports/harness/run-scenario/startup-smoke/terminal_report.json")),
        "terminal report artifact should use the stable harness path contract"
    );
    assert!(
        report.artifacts.iter().any(|artifact| artifact.role == "tweak_pack_source"
            && artifact.path == "tweak_packs/release/hero_forest.ron"),
        "startup scenario reports should record the applied tweak pack artifact"
    );
    assert!(
        report.artifacts.iter().any(|artifact| artifact.role == "metrics_summary"
            && artifact
                .path
                .ends_with("reports/harness/run-scenario/startup-smoke/metrics_summary.json")),
        "scenario runs should publish a stable metrics summary artifact"
    );
    assert!(
        report.artifacts.iter().any(|artifact| artifact.role == "trace_log"
            && artifact.path.ends_with("reports/harness/run-scenario/startup-smoke/trace.jsonl")),
        "scenario runs should publish a stable trace log artifact"
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
    assert_eq!(first_report.assertions, second_report.assertions);
    assert_eq!(first_report.metrics.frames_requested, second_report.metrics.frames_requested);
    assert_eq!(first_report.metrics.frames_simulated, second_report.metrics.frames_simulated);
    assert_eq!(first_report.metrics.simulation_rate_hz, second_report.metrics.simulation_rate_hz);
    assert_eq!(first_report.metrics.spawned_actor_count, second_report.metrics.spawned_actor_count);
    assert_eq!(
        first_report.metrics.scripted_input_count,
        second_report.metrics.scripted_input_count
    );
    assert_eq!(first_report.metrics.applied_input_count, second_report.metrics.applied_input_count);
    let first_telemetry = first_report
        .metrics
        .telemetry_summary
        .as_ref()
        .expect("telemetry summary should be present");
    let second_telemetry = second_report
        .metrics
        .telemetry_summary
        .as_ref()
        .expect("telemetry summary should be present");
    assert_eq!(first_telemetry.frame_count, second_telemetry.frame_count);
    assert_eq!(first_telemetry.entity_count, second_telemetry.entity_count);
    assert_eq!(first_telemetry.memory_bytes, second_telemetry.memory_bytes);
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
    assert_eq!(
        report.metrics.telemetry_summary.as_ref().map(|summary| summary.frame_count),
        Some(1)
    );
    assert!(
        report
            .result
            .details
            .as_deref()
            .is_some_and(|details| details.contains("world.actor_count")),
        "the failure detail should identify the failing metric"
    );
}
