#![forbid(unsafe_code)]

use std::path::{Component, Path, PathBuf};

use wr_core::{CrateBoundary, CrateEntryPoint, TweakPack, load_tweak_pack_ron};
use wr_game::HeadlessScenarioSummary;
use wr_telemetry::{PlatformMetadata, RunMetadata, RunTimestamps, SeedInfo, artifact_component};
use wr_tools_harness::{
    ArtifactDescriptor, ArtifactLayout, FailureKind, HARNESS_SCHEMA_VERSION, HarnessStatus,
    ResultEnvelope, ScenarioExecutionMetrics, ScenarioExecutionReport, TERMINAL_REPORT_FILENAME,
    load_scenario_request_ron, write_json_artifact_at,
};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_headless", CrateBoundary::AppShell, true)
}

pub const fn target_runtime() -> CrateEntryPoint {
    wr_game::init_entrypoint()
}

pub const RUN_SCENARIO_COMMAND_NAME: &str = "run-scenario";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessRunOutcome {
    pub terminal_report_path: PathBuf,
    pub succeeded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HeadlessRunOptions {
    scenario_path: PathBuf,
    output_root: PathBuf,
    run_id: Option<String>,
}

pub fn run(mut args: impl Iterator<Item = String>) -> i32 {
    match run_scenario_command(&mut args) {
        Ok(outcome) => {
            println!("{}", outcome.terminal_report_path.display());
            if outcome.succeeded { 0 } else { 1 }
        }
        Err(error) => {
            eprintln!("headless scenario runner failed: {error}");
            1
        }
    }
}

pub fn run_scenario_command(
    args: impl Iterator<Item = String>,
) -> Result<HeadlessRunOutcome, String> {
    let options = HeadlessRunOptions::parse(args)?;
    execute_run(options)
}

impl HeadlessRunOptions {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut scenario_path = None;
        let mut output_root = PathBuf::from(".");
        let mut run_id = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scenario" => {
                    let value = args
                        .next()
                        .ok_or_else(|| String::from("expected a value after --scenario"))?;
                    scenario_path = Some(PathBuf::from(value));
                }
                "--output-root" => {
                    let value = args
                        .next()
                        .ok_or_else(|| String::from("expected a value after --output-root"))?;
                    output_root = PathBuf::from(value);
                }
                "--run-id" => {
                    run_id = Some(
                        args.next()
                            .ok_or_else(|| String::from("expected a value after --run-id"))?,
                    );
                }
                value if scenario_path.is_none() => {
                    scenario_path = Some(PathBuf::from(value));
                }
                other => {
                    return Err(format!(
                        "unsupported argument `{other}` for wr_headless; supported flags: --scenario, --output-root, --run-id"
                    ));
                }
            }
        }

        let scenario_path = scenario_path
            .ok_or_else(|| String::from("missing scenario path; pass --scenario <path>"))?;

        Ok(Self { scenario_path, output_root, run_id })
    }
}

fn execute_run(options: HeadlessRunOptions) -> Result<HeadlessRunOutcome, String> {
    let scenario_label = artifact_component(options.scenario_path.to_string_lossy().as_ref());
    let run_id = options.run_id.unwrap_or_else(|| format!("{scenario_label}-{}", now_unix_ms()));
    let layout = ArtifactLayout::new(RUN_SCENARIO_COMMAND_NAME, &run_id);
    let started_at = now_unix_ms();
    let cwd =
        std::env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))?;
    let git_sha = current_git_sha().unwrap_or_else(|_| String::from("<unknown>"));

    let load_result = load_scenario_request_ron(&options.scenario_path);
    let (_raw_seed, scenario_path, tweak_pack_path, summary, notes) = match load_result {
        Ok(scenario) => {
            if let Err(error) =
                validate_declared_scenario_path(&options.scenario_path, &scenario.scenario_path)
            {
                (
                    scenario.seed,
                    options.scenario_path.to_string_lossy().into_owned(),
                    scenario.tweak_pack_path,
                    failure_summary("Scenario could not be loaded.", error),
                    None,
                )
            } else {
                let scenario_note =
                    format!("Scenario source loaded from {}.", options.scenario_path.display());
                let tweak_pack_path = scenario.tweak_pack_path.clone();
                match load_optional_tweak_pack(scenario.tweak_pack_path.as_deref()) {
                    Ok(tweak_pack) => {
                        let mut notes = vec![scenario_note];
                        if let Some(path) = tweak_pack_path.as_deref() {
                            notes.push(format!("Tweak pack applied from {path}."));
                        }

                        (
                            scenario.seed.clone(),
                            scenario.scenario_path.clone(),
                            tweak_pack_path,
                            wr_game::run_headless_scenario_with_tweak_pack(
                                &scenario,
                                tweak_pack.as_ref(),
                            ),
                            Some(notes),
                        )
                    }
                    Err(error) => (
                        scenario.seed.clone(),
                        scenario.scenario_path.clone(),
                        tweak_pack_path,
                        failure_summary("Scenario could not be executed.", error),
                        Some(vec![scenario_note]),
                    ),
                }
            }
        }
        Err(error) => (
            SeedInfo {
                label: "unknown".to_owned(),
                value_hex: "0x0000000000000000".to_owned(),
                stream: Some("load_failure".to_owned()),
                derivations: Vec::new(),
                config_pack: None,
            },
            options.scenario_path.to_string_lossy().into_owned(),
            None,
            failure_summary("Scenario could not be loaded.", error.to_string()),
            None,
        ),
    };

    let completed_at = now_unix_ms();
    let metadata = RunMetadata::new(
        RUN_SCENARIO_COMMAND_NAME,
        &run_id,
        git_sha,
        cwd.display().to_string(),
        PlatformMetadata::current(),
        RunTimestamps { started_at_unix_ms: started_at, completed_at_unix_ms: completed_at },
    );

    let mut artifacts = vec![ArtifactDescriptor {
        role: "terminal_report".to_owned(),
        path: layout.terminal_report_path_string(),
        media_type: "application/json".to_owned(),
    }];
    if Path::new(&scenario_path).exists() {
        artifacts.push(ArtifactDescriptor {
            role: "scenario_source".to_owned(),
            path: scenario_path.clone(),
            media_type: "text/ron".to_owned(),
        });
    }
    if let Some(tweak_pack_path) = &tweak_pack_path
        && Path::new(tweak_pack_path).exists()
    {
        artifacts.push(ArtifactDescriptor {
            role: "tweak_pack_source".to_owned(),
            path: tweak_pack_path.clone(),
            media_type: "text/ron".to_owned(),
        });
    }

    let report = ScenarioExecutionReport {
        schema_version: HARNESS_SCHEMA_VERSION.to_owned(),
        metadata,
        seed: summary.report_seed,
        scenario_path,
        result: summary.result.clone(),
        metrics: summary.metrics,
        determinism_hash: summary.determinism_hash,
        assertions: summary.assertions,
        artifacts,
        notes: merge_notes(summary.notes, notes),
    };

    let terminal_report_path =
        write_json_artifact_at(&options.output_root, &layout, TERMINAL_REPORT_FILENAME, &report)
            .map_err(|error| format!("failed to write terminal report: {error}"))?;

    Ok(HeadlessRunOutcome {
        terminal_report_path,
        succeeded: report.result.status == HarnessStatus::Passed,
    })
}

fn validate_declared_scenario_path(loaded_path: &Path, declared_path: &str) -> Result<(), String> {
    let loaded = loaded_path.canonicalize().map_err(|error| {
        format!("failed to canonicalize loaded scenario path `{}`: {error}", loaded_path.display())
    })?;
    let declared_path = Path::new(declared_path);
    let declared = if declared_path.is_absolute() || declared_path.exists() {
        declared_path.canonicalize().map_err(|error| {
            format!(
                "declared scenario_path `{}` could not be resolved: {error}",
                declared_path.display()
            )
        })?
    } else {
        normalize_relative_path(declared_path)
    };

    if loaded == declared || (!declared.is_absolute() && loaded.ends_with(&declared)) {
        Ok(())
    } else {
        Err(format!(
            "declared scenario_path `{}` does not match loaded file `{}`",
            declared.display(),
            loaded_path.display()
        ))
    }
}

fn normalize_relative_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }

    normalized
}

fn failure_summary(summary: &str, details: String) -> HeadlessScenarioSummary {
    HeadlessScenarioSummary {
        report_seed: SeedInfo {
            label: "unknown".to_owned(),
            value_hex: "0x0000000000000000".to_owned(),
            stream: Some("load_failure".to_owned()),
            derivations: Vec::new(),
            config_pack: None,
        },
        result: ResultEnvelope {
            status: HarnessStatus::Failed,
            summary: summary.to_owned(),
            failure_kind: Some(FailureKind::ScenarioFailed),
            details: Some(details),
        },
        metrics: ScenarioExecutionMetrics {
            frames_requested: 0,
            frames_simulated: 0,
            simulation_rate_hz: 0,
            spawned_actor_count: 0,
            scripted_input_count: 0,
            applied_input_count: 0,
        },
        assertions: Vec::new(),
        determinism_hash: "0x0000000000000000".to_owned(),
        notes: Some(vec![
            "A terminal report was still emitted so automation can classify the failure."
                .to_owned(),
        ]),
    }
}

fn load_optional_tweak_pack(path: Option<&str>) -> Result<Option<TweakPack>, String> {
    path.map(|path| {
        load_tweak_pack_ron(path)
            .map_err(|error| format!("failed to load tweak pack `{path}`: {error}"))
    })
    .transpose()
}

fn merge_notes(
    primary: Option<Vec<String>>,
    secondary: Option<Vec<String>>,
) -> Option<Vec<String>> {
    let mut notes = primary.unwrap_or_default();
    notes.extend(secondary.unwrap_or_default());

    if notes.is_empty() { None } else { Some(notes) }
}

fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn current_git_sha() -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| format!("failed to run git rev-parse HEAD: {error}"))?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map(|sha| sha.trim().to_owned())
            .map_err(|error| format!("git rev-parse returned non-utf8 output: {error}"))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn declared_scenario_path_must_match_loaded_file() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let actual_path = temp.path().join("startup.ron");
        let declared_path = temp.path().join("other.ron");

        fs::write(&actual_path, "()\n").expect("actual file should be written");
        fs::write(&declared_path, "()\n").expect("declared file should be written");

        let error = validate_declared_scenario_path(&actual_path, &declared_path.to_string_lossy())
            .expect_err("mismatched declared path should fail");

        assert!(error.contains("does not match loaded file"));
    }

    #[test]
    fn declared_scenario_path_accepts_same_file_with_relative_segments() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root should exist");
        let loaded = workspace_root.join("scenarios/smoke/startup.ron");
        let declared = "./scenarios/smoke/startup.ron";

        validate_declared_scenario_path(&loaded, declared)
            .expect("same file with relative segments should validate");
    }
}
