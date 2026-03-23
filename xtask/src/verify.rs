use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use tracing::{info, warn};
use wr_telemetry::{
    PlatformMetadata, RunMetadata, RunTimestamps, SeedInfo, VerificationStepRecord,
    VerificationStepStatus, artifact_component,
};
use wr_tools_harness::{
    ArtifactDescriptor, ArtifactLayout, FailureKind, HarnessStatus, ResultEnvelope,
    TestResultBundle, TestSuiteResult, write_test_result_bundle_at,
};

use crate::util::{current_git_sha, now_unix_ms};

const VERIFY_COMMAND_NAME: &str = "verify";
const VERIFY_STEP_RECORDS_FILENAME: &str = "verify_steps.json";
const TRACE_LOG_FILENAME: &str = "trace.jsonl";

pub struct VerifyOutcome {
    pub terminal_report_path: PathBuf,
    pub succeeded: bool,
}

pub fn run(args: impl Iterator<Item = String>) -> Result<VerifyOutcome, String> {
    let options = VerifyOptions::parse(args)?;
    let run_id = options.run_id.unwrap_or_else(|| format!("verify-{}", now_unix_ms().unwrap_or(0)));
    let layout = ArtifactLayout::new(VERIFY_COMMAND_NAME, &run_id);
    let run_dir = layout.run_directory();

    fs::create_dir_all(&run_dir)
        .map_err(|error| format!("failed to create verify run directory: {error}"))?;

    let trace_path = run_dir.join(TRACE_LOG_FILENAME);
    let trace_file = fs::File::create(&trace_path)
        .map_err(|error| format!("failed to create trace log: {error}"))?;
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .with_writer(move || {
            trace_file
                .try_clone()
                .expect("verify trace file should remain cloneable for the subscriber")
        })
        .finish();
    let _subscriber_guard = tracing::subscriber::set_default(subscriber);

    let started_at = now_unix_ms()?;
    let git_sha = current_git_sha()?;
    let cwd =
        std::env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))?;

    info!(run_id = %run_id, cwd = %cwd.display(), "starting verification stack");

    let mut state = VerifyRunState::new(
        layout,
        run_id,
        started_at,
        git_sha,
        cwd,
        vec![artifact_descriptor("trace_log", &trace_path, "application/x-ndjson")],
    );

    if state.record_suite_result(verify_process_contract(&run_dir))? {
        return state.finalize();
    }

    if state.record_suite_result(verify_fmt(&run_dir))? {
        return state.finalize();
    }

    if state.record_suite_result(verify_clippy(&run_dir))? {
        return state.finalize();
    }

    if state.record_suite_result(verify_nextest(&run_dir))? {
        return state.finalize();
    }

    if state.record_suite_result(verify_benchmark(&run_dir))? {
        return state.finalize();
    }

    state.finalize()
}

#[derive(Debug, Default)]
struct VerifyOptions {
    run_id: Option<String>,
}

impl VerifyOptions {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut options = Self::default();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--run-id" => {
                    options.run_id = Some(
                        args.next()
                            .ok_or_else(|| String::from("expected a value after --run-id"))?,
                    );
                }
                other => {
                    return Err(format!(
                        "unsupported argument `{other}` for verify; supported flags: --run-id"
                    ));
                }
            }
        }

        Ok(options)
    }
}

struct VerifyRunState {
    layout: ArtifactLayout,
    run_id: String,
    started_at: u64,
    git_sha: String,
    cwd: PathBuf,
    steps: Vec<VerificationStepRecord>,
    suites: Vec<TestSuiteResult>,
    artifacts: Vec<ArtifactDescriptor>,
    failure_kind: Option<FailureKind>,
    failure_details: Option<String>,
}

impl VerifyRunState {
    fn new(
        layout: ArtifactLayout,
        run_id: String,
        started_at: u64,
        git_sha: String,
        cwd: PathBuf,
        artifacts: Vec<ArtifactDescriptor>,
    ) -> Self {
        Self {
            layout,
            run_id,
            started_at,
            git_sha,
            cwd,
            steps: Vec::new(),
            suites: Vec::new(),
            artifacts,
            failure_kind: None,
            failure_details: None,
        }
    }

    fn record_suite_result(
        &mut self,
        result: Result<Option<SuiteExecution>, String>,
    ) -> Result<bool, String> {
        match result {
            Ok(Some(execution)) => {
                let SuiteExecution {
                    record,
                    counts,
                    extra_artifacts: suite_artifacts,
                    failure_kind: suite_failure_kind,
                    failure_details: suite_failure_details,
                } = execution;

                if record.status != VerificationStepStatus::Skipped {
                    info!(
                        suite = %record.suite_name,
                        status = ?record.status,
                        duration_ms = record.duration_ms,
                        "verification suite completed"
                    );
                }

                self.artifacts.extend(suite_artifacts);
                self.suites.push(TestSuiteResult {
                    name: record.suite_name.clone(),
                    passed: counts.passed,
                    failed: counts.failed,
                    ignored: counts.ignored,
                    duration_ms: record.duration_ms,
                    stdout_artifact: record.stdout_artifact.clone(),
                    stderr_artifact: record.stderr_artifact.clone(),
                });
                let failed_step = record.status == VerificationStepStatus::Failed;
                self.steps.push(record);

                if failed_step {
                    self.failure_kind = Some(suite_failure_kind);
                    self.failure_details = suite_failure_details;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Ok(None) => Ok(false),
            Err(error) => {
                warn!(error = %error, "verification stack encountered an unrecoverable error");
                self.failure_kind = Some(FailureKind::BuildFailed);
                self.failure_details = Some(error);
                Ok(true)
            }
        }
    }

    fn finalize(mut self) -> Result<VerifyOutcome, String> {
        let run_dir = self.layout.run_directory();
        let step_records_path = write_step_records(&run_dir, &self.steps)?;
        self.artifacts.push(artifact_descriptor(
            "verification_step_records",
            &step_records_path,
            "application/json",
        ));

        let completed_at = now_unix_ms()?;
        let metadata = RunMetadata::new(
            VERIFY_COMMAND_NAME,
            self.run_id,
            self.git_sha,
            self.cwd.display().to_string(),
            PlatformMetadata::current(),
            RunTimestamps {
                started_at_unix_ms: self.started_at,
                completed_at_unix_ms: completed_at,
            },
        );

        let summary = if self.failure_kind.is_none() {
            "Workspace verification stack passed.".to_owned()
        } else {
            "Workspace verification stack failed.".to_owned()
        };

        let succeeded = self.failure_kind.is_none();
        let result = ResultEnvelope {
            status: if self.failure_kind.is_none() {
                HarnessStatus::Passed
            } else {
                HarnessStatus::Failed
            },
            summary,
            failure_kind: self.failure_kind,
            details: self.failure_details,
        };

        let terminal_report_path =
            write_verify_bundle(&self.layout, metadata, self.suites, self.artifacts, result)?;

        Ok(VerifyOutcome { terminal_report_path, succeeded })
    }
}

struct SuiteExecution {
    record: VerificationStepRecord,
    counts: SuiteCounts,
    extra_artifacts: Vec<ArtifactDescriptor>,
    failure_kind: FailureKind,
    failure_details: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct SuiteCounts {
    passed: u32,
    failed: u32,
    ignored: u32,
}

impl SuiteCounts {
    const fn command_level(success: bool) -> Self {
        if success {
            Self { passed: 1, failed: 0, ignored: 0 }
        } else {
            Self { passed: 0, failed: 1, ignored: 0 }
        }
    }
}

struct CapturedCommand {
    record: VerificationStepRecord,
    stdout_path: PathBuf,
    stdout: Vec<u8>,
    extra_artifacts: Vec<ArtifactDescriptor>,
}

fn verify_process_contract(run_dir: &Path) -> Result<Option<SuiteExecution>, String> {
    let captured = run_command_capture(
        "process-contract",
        "python3",
        &["docs/process/validate_process_contract.py"],
        run_dir,
        "stdout.json",
        &[],
    )?;
    let success = captured.record.status == VerificationStepStatus::Passed;

    Ok(Some(SuiteExecution {
        record: captured.record,
        counts: SuiteCounts::command_level(success),
        extra_artifacts: captured.extra_artifacts,
        failure_kind: FailureKind::TestFailed,
        failure_details: (!success).then(|| {
            format!(
                "bootstrap process contract validation failed; inspect {}",
                path_string(&captured.stdout_path)
            )
        }),
    }))
}

fn verify_fmt(run_dir: &Path) -> Result<Option<SuiteExecution>, String> {
    let captured =
        run_command_capture("fmt", "cargo", &["fmt", "--check"], run_dir, "stdout.log", &[])?;
    let success = captured.record.status == VerificationStepStatus::Passed;

    Ok(Some(SuiteExecution {
        record: captured.record,
        counts: SuiteCounts::command_level(success),
        extra_artifacts: captured.extra_artifacts,
        failure_kind: FailureKind::TestFailed,
        failure_details: (!success).then(|| {
            String::from("formatting check failed; inspect the fmt suite artifacts for details")
        }),
    }))
}

fn verify_clippy(run_dir: &Path) -> Result<Option<SuiteExecution>, String> {
    let captured = run_command_capture(
        "clippy",
        "cargo",
        &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
        run_dir,
        "stdout.log",
        &[],
    )?;
    let success = captured.record.status == VerificationStepStatus::Passed;

    Ok(Some(SuiteExecution {
        record: captured.record,
        counts: SuiteCounts::command_level(success),
        extra_artifacts: captured.extra_artifacts,
        failure_kind: FailureKind::TestFailed,
        failure_details: (!success)
            .then(|| String::from("clippy failed; inspect the clippy suite artifacts for details")),
    }))
}

fn verify_nextest(run_dir: &Path) -> Result<Option<SuiteExecution>, String> {
    ensure_cargo_nextest_available()?;

    let junit_source = Path::new("target").join("nextest").join("ci").join("junit.xml");
    if junit_source.is_file() {
        fs::remove_file(&junit_source)
            .map_err(|error| format!("failed to clear stale nextest JUnit artifact: {error}"))?;
    }

    let list = run_command_capture(
        "nextest-list",
        "cargo",
        &["nextest", "list", "--workspace", "--profile", "ci", "--message-format", "json"],
        run_dir,
        "stdout.jsonl",
        &[],
    )?;
    let mut extra_artifacts = list.extra_artifacts;
    if list.record.status == VerificationStepStatus::Failed {
        return Ok(Some(SuiteExecution {
            record: list.record,
            counts: SuiteCounts::command_level(false),
            extra_artifacts,
            failure_kind: FailureKind::TestFailed,
            failure_details: Some(String::from(
                "nextest test inventory export failed; inspect the nextest-list artifacts",
            )),
        }));
    }

    let run = run_command_capture(
        "nextest",
        "cargo",
        &[
            "nextest",
            "run",
            "--workspace",
            "--profile",
            "ci",
            "--failure-output",
            "immediate-final",
            "--success-output",
            "never",
            "--message-format",
            "libtest-json-plus",
        ],
        run_dir,
        "events.jsonl",
        &[
            ("INSTA_OUTPUT", "diff"),
            ("INSTA_UPDATE", "new"),
            ("NEXTEST_EXPERIMENTAL_LIBTEST_JSON", "1"),
        ],
    )?;
    extra_artifacts.extend(run.extra_artifacts);

    let mut counts = parse_nextest_suite_counts(&run.stdout).unwrap_or_else(|| {
        SuiteCounts::command_level(run.record.status == VerificationStepStatus::Passed)
    });

    if junit_source.is_file() {
        let junit_dest = run_dir.join("nextest-junit.xml");
        fs::copy(&junit_source, &junit_dest)
            .map_err(|error| format!("failed to copy nextest JUnit artifact: {error}"))?;
        extra_artifacts.push(artifact_descriptor("nextest_junit", &junit_dest, "application/xml"));
    }

    if run.record.status == VerificationStepStatus::Failed && counts.failed == 0 {
        counts.failed = 1;
    }

    Ok(Some(SuiteExecution {
        record: run.record,
        counts,
        extra_artifacts,
        failure_kind: FailureKind::TestFailed,
        failure_details: (counts.failed > 0).then(|| {
            String::from("nextest run failed; inspect the nextest JSON and JUnit artifacts")
        }),
    }))
}

fn verify_benchmark(run_dir: &Path) -> Result<Option<SuiteExecution>, String> {
    let criterion_dir = Path::new("target").join("criterion");
    if criterion_dir.is_dir() {
        fs::remove_dir_all(&criterion_dir)
            .map_err(|error| format!("failed to clear stale criterion artifacts: {error}"))?;
    }

    let captured = run_command_capture(
        "criterion-bench",
        "cargo",
        &["bench", "-p", "wr_telemetry", "--bench", "artifact_component", "--", "--noplot"],
        run_dir,
        "stdout.log",
        &[],
    )?;
    let success = captured.record.status == VerificationStepStatus::Passed;
    let mut extra_artifacts = captured.extra_artifacts;
    extra_artifacts.extend(copy_criterion_estimates(run_dir)?);

    Ok(Some(SuiteExecution {
        record: captured.record,
        counts: SuiteCounts::command_level(success),
        extra_artifacts,
        failure_kind: FailureKind::TestFailed,
        failure_details: (!success).then(|| {
            String::from("criterion benchmark group failed; inspect the benchmark artifacts")
        }),
    }))
}

fn run_command_capture(
    suite_name: &str,
    program: &str,
    args: &[&str],
    run_dir: &Path,
    stdout_filename: &str,
    envs: &[(&str, &str)],
) -> Result<CapturedCommand, String> {
    let suite_slug = artifact_component(suite_name);
    let stdout_path = run_dir.join(format!("{suite_slug}.{stdout_filename}"));
    let stderr_path = run_dir.join(format!("{suite_slug}.stderr.log"));
    let full_command: Vec<String> = std::iter::once(program.to_owned())
        .chain(args.iter().map(|arg| (*arg).to_owned()))
        .collect();

    info!(suite = suite_name, command = ?full_command, "starting verification suite");

    let started = Instant::now();
    let output = Command::new(program)
        .args(args)
        .envs(envs.iter().copied())
        .output()
        .map_err(|error| format!("failed to invoke `{program}` for `{suite_name}`: {error}"))?;
    let duration_ms = u64::try_from(started.elapsed().as_millis())
        .map_err(|_| format!("duration overflow for verification suite `{suite_name}`"))?;

    fs::write(&stdout_path, &output.stdout)
        .map_err(|error| format!("failed to write stdout for `{suite_name}`: {error}"))?;
    fs::write(&stderr_path, &output.stderr)
        .map_err(|error| format!("failed to write stderr for `{suite_name}`: {error}"))?;

    let mut record = VerificationStepRecord::new(
        suite_name,
        full_command,
        if output.status.success() {
            VerificationStepStatus::Passed
        } else {
            VerificationStepStatus::Failed
        },
        duration_ms,
    );
    record.exit_code = output.status.code();
    record.stdout_artifact = Some(path_string(&stdout_path));
    record.stderr_artifact = Some(path_string(&stderr_path));

    let media_type = if stdout_filename.ends_with(".json") || stdout_filename.ends_with(".jsonl") {
        if stdout_filename.ends_with(".jsonl") {
            "application/x-ndjson"
        } else {
            "application/json"
        }
    } else {
        "text/plain"
    };

    Ok(CapturedCommand {
        record,
        stdout_path: stdout_path.clone(),
        stdout: output.stdout,
        extra_artifacts: vec![
            artifact_descriptor(&format!("{suite_slug}_stdout"), &stdout_path, media_type),
            artifact_descriptor(&format!("{suite_slug}_stderr"), &stderr_path, "text/plain"),
        ],
    })
}

fn parse_nextest_suite_counts(stdout: &[u8]) -> Option<SuiteCounts> {
    let mut counts = SuiteCounts { passed: 0, failed: 0, ignored: 0 };
    let mut saw_completed_suite = false;
    let text = std::str::from_utf8(stdout).ok()?;

    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("type").and_then(|field| field.as_str()) != Some("suite") {
            continue;
        }
        let Some(event) = value.get("event").and_then(|field| field.as_str()) else {
            continue;
        };
        if !matches!(event, "ok" | "failed") {
            continue;
        }

        saw_completed_suite = true;
        counts.passed += value.get("passed").and_then(|field| field.as_u64()).unwrap_or(0) as u32;
        counts.failed += value.get("failed").and_then(|field| field.as_u64()).unwrap_or(0) as u32;
        counts.ignored += value.get("ignored").and_then(|field| field.as_u64()).unwrap_or(0) as u32;
    }

    saw_completed_suite.then_some(counts)
}

fn ensure_cargo_nextest_available() -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["nextest", "--version"])
        .output()
        .map_err(|error| format!("failed to check cargo-nextest availability: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from(
            "cargo-nextest is required for `cargo xtask verify`; install it with `cargo install cargo-nextest --locked`",
        ))
    }
}

fn copy_criterion_estimates(run_dir: &Path) -> Result<Vec<ArtifactDescriptor>, String> {
    let source_root = Path::new("target").join("criterion");
    if !source_root.is_dir() {
        return Ok(Vec::new());
    }

    let destination_root = run_dir.join("criterion");
    let estimate_files = collect_estimate_files(&source_root)?;
    let mut artifacts = Vec::new();

    for source in estimate_files {
        let relative = source
            .strip_prefix(&source_root)
            .map_err(|error| format!("failed to relativize criterion artifact: {error}"))?;
        let destination = destination_root.join(relative);

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("failed to create criterion artifact directory: {error}")
            })?;
        }

        fs::copy(&source, &destination)
            .map_err(|error| format!("failed to copy criterion estimate artifact: {error}"))?;
        artifacts.push(artifact_descriptor(
            "criterion_estimates",
            &destination,
            "application/json",
        ));
    }

    Ok(artifacts)
}

fn collect_estimate_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).map_err(|error| {
            format!("failed to read criterion directory {}: {error}", directory.display())
        })? {
            let entry = entry
                .map_err(|error| format!("failed to inspect criterion directory entry: {error}"))?;
            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|name| name.to_str()) == Some("estimates.json") {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn write_step_records(run_dir: &Path, steps: &[VerificationStepRecord]) -> Result<PathBuf, String> {
    fs::create_dir_all(run_dir)
        .map_err(|error| format!("failed to create verify artifact directory: {error}"))?;
    let path = run_dir.join(VERIFY_STEP_RECORDS_FILENAME);
    let mut json = serde_json::to_vec_pretty(steps)
        .map_err(|error| format!("failed to serialize steps: {error}"))?;
    json.push(b'\n');
    fs::write(&path, json)
        .map_err(|error| format!("failed to write verify step records: {error}"))?;
    Ok(path)
}

fn write_verify_bundle(
    layout: &ArtifactLayout,
    metadata: RunMetadata,
    suites: Vec<TestSuiteResult>,
    artifacts: Vec<ArtifactDescriptor>,
    result: ResultEnvelope,
) -> Result<PathBuf, String> {
    write_verify_bundle_at(Path::new("."), layout, metadata, suites, artifacts, result)
}

fn write_verify_bundle_at(
    root: &Path,
    layout: &ArtifactLayout,
    metadata: RunMetadata,
    suites: Vec<TestSuiteResult>,
    artifacts: Vec<ArtifactDescriptor>,
    result: ResultEnvelope,
) -> Result<PathBuf, String> {
    let bundle = TestResultBundle {
        schema_version: "wr_harness/v1".to_owned(),
        metadata,
        seed: SeedInfo {
            label: "verification_stack".to_owned(),
            value_hex: "0x00000000".to_owned(),
            stream: Some("repo-automation".to_owned()),
            derivations: Vec::new(),
            config_pack: None,
        },
        result,
        suites,
        artifacts,
        notes: Some(vec![
            "cargo xtask verify is the repo-standard wrapper for fmt, clippy, nextest, and the selected benchmark group."
                .to_owned(),
            "nextest snapshot failures are configured to emit human-readable diffs and .snap.new review artifacts."
                .to_owned(),
        ]),
    };

    write_test_result_bundle_at(root, layout, &bundle).map_err(|error| error.to_string())
}

fn artifact_descriptor(role: &str, path: &Path, media_type: &str) -> ArtifactDescriptor {
    ArtifactDescriptor {
        role: role.to_owned(),
        path: path_string(path),
        media_type: media_type.to_owned(),
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;
    use wr_tools_harness::{ArtifactLayout, HarnessStatus};

    use super::*;

    #[test]
    fn verify_bundle_and_step_records_land_under_reports() {
        let temp = tempdir().expect("temp directory should exist");
        let layout = ArtifactLayout::new(VERIFY_COMMAND_NAME, "unit-test");
        let run_dir = temp.path().join(layout.run_directory());
        let step_records = vec![VerificationStepRecord::new(
            "fmt",
            vec!["cargo".to_owned(), "fmt".to_owned(), "--check".to_owned()],
            VerificationStepStatus::Passed,
            5,
        )];
        let step_records_path =
            write_step_records(&run_dir, &step_records).expect("steps should write");
        let report_path = write_verify_bundle_at(
            temp.path(),
            &layout,
            RunMetadata::new(
                VERIFY_COMMAND_NAME,
                "unit-test",
                "0123456789abcdef0123456789abcdef01234567",
                temp.path().display().to_string(),
                PlatformMetadata::current(),
                RunTimestamps { started_at_unix_ms: 1, completed_at_unix_ms: 2 },
            ),
            vec![TestSuiteResult {
                name: "fmt".to_owned(),
                passed: 1,
                failed: 0,
                ignored: 0,
                duration_ms: 5,
                stdout_artifact: None,
                stderr_artifact: None,
            }],
            vec![artifact_descriptor(
                "verification_step_records",
                &step_records_path,
                "application/json",
            )],
            ResultEnvelope {
                status: HarnessStatus::Passed,
                summary: "ok".to_owned(),
                failure_kind: None,
                details: None,
            },
        )
        .expect("terminal report should write");

        assert!(step_records_path.is_file());
        assert!(report_path.is_file());
        assert_eq!(
            step_records_path,
            temp.path()
                .join("reports")
                .join("harness")
                .join("verify")
                .join("unit-test")
                .join(VERIFY_STEP_RECORDS_FILENAME)
        );
        assert_eq!(
            report_path,
            temp.path()
                .join("reports")
                .join("harness")
                .join("verify")
                .join("unit-test")
                .join("terminal_report.json")
        );

        let report = fs::read_to_string(report_path).expect("terminal report should be readable");
        assert!(report.contains("\"command_name\": \"verify\""));
    }
}
