use std::path::{Path, PathBuf};

use serde::Serialize;
use wr_headless::capture_request_for_scenario;
use wr_render_wgpu::render_offscreen_png;
use wr_telemetry::{PlatformMetadata, RunMetadata, RunTimestamps, artifact_component};
use wr_tools_harness::{
    ArtifactDescriptor, ArtifactLayout, CommandExecutionReport, FailureKind,
    HARNESS_SCHEMA_VERSION, HarnessStatus, ResultEnvelope, TERMINAL_REPORT_FILENAME,
    load_scenario_request_ron, write_json_artifact,
};

use crate::util::{current_git_sha, now_unix_ms};

const CAPTURE_IMAGE_FILENAME: &str = "frame.png";
const CAPTURE_METADATA_FILENAME: &str = "capture_metadata.json";

pub struct BootstrapCommandOutcome {
    pub terminal_report_path: PathBuf,
    pub succeeded: bool,
}

pub fn run_capture(args: impl Iterator<Item = String>) -> Result<BootstrapCommandOutcome, String> {
    let options = ScenarioCommandOptions::parse("capture", args)?;
    let run_id =
        options.run_id.unwrap_or_else(|| default_run_id("capture", &options.scenario_path));
    let layout = ArtifactLayout::new("capture", &run_id);
    let started_at = now_unix_ms()?;
    let git_sha = current_git_sha().unwrap_or_else(|_| String::from("<unknown>"));
    let cwd =
        std::env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))?;

    let mut artifacts = vec![
        ArtifactDescriptor {
            role: "terminal_report".to_owned(),
            path: layout.terminal_report_path_string(),
            media_type: "application/json".to_owned(),
        },
        source_artifact("scenario_source", &options.scenario_path, "text/ron"),
    ];

    let report_result = match load_scenario_request_ron(&options.scenario_path) {
        Ok(scenario) => {
            let request = capture_request_for_scenario(&scenario);
            let capture_path = layout.run_directory().join(CAPTURE_IMAGE_FILENAME);
            match render_offscreen_png(&request, &capture_path) {
                Ok(outcome) => {
                    let metadata = CaptureMetadataArtifact {
                        adapter: outcome.adapter,
                        frame: outcome.frame.clone(),
                    };
                    write_json_artifact(&layout, CAPTURE_METADATA_FILENAME, &metadata)
                        .map_err(|error| format!("failed to write capture metadata: {error}"))?;
                    artifacts.push(ArtifactDescriptor {
                        role: "capture_png".to_owned(),
                        path: capture_path.to_string_lossy().into_owned(),
                        media_type: "image/png".to_owned(),
                    });
                    artifacts.push(ArtifactDescriptor {
                        role: "capture_metadata".to_owned(),
                        path: layout
                            .run_directory()
                            .join(CAPTURE_METADATA_FILENAME)
                            .to_string_lossy()
                            .into_owned(),
                        media_type: "application/json".to_owned(),
                    });

                    ReportBuildResult {
                        result: ResultEnvelope {
                            status: HarnessStatus::Passed,
                            summary: format!(
                                "Captured a {}x{} offscreen PNG for scenario `{}`.",
                                metadata.frame.size.width,
                                metadata.frame.size.height,
                                scenario.scenario_path
                            ),
                            failure_kind: None,
                            details: None,
                        },
                        notes: Some(vec![
                            format!(
                                "Adapter backend={} name={} device_type={}.",
                                metadata.adapter.backend,
                                metadata.adapter.name,
                                metadata.adapter.device_type
                            ),
                            format!(
                                "Color space={} non_empty_pixels={}.",
                                metadata.frame.color_space.as_str(),
                                metadata.frame.non_empty_pixels
                            ),
                        ]),
                        succeeded: true,
                    }
                }
                Err(error) => ReportBuildResult {
                    result: ResultEnvelope {
                        status: HarnessStatus::Failed,
                        summary: format!(
                            "Failed to capture an offscreen PNG for scenario `{}`.",
                            scenario.scenario_path
                        ),
                        failure_kind: Some(FailureKind::RuntimeCrash),
                        details: Some(error),
                    },
                    notes: Some(vec![
                        "Offscreen capture uses the deterministic seed-derived clear color until scene extraction lands.".to_owned(),
                    ]),
                    succeeded: false,
                },
            }
        }
        Err(error) => ReportBuildResult {
            result: ResultEnvelope {
                status: HarnessStatus::Failed,
                summary: format!("Failed to load capture scenario `{}`.", options.scenario_path),
                failure_kind: Some(FailureKind::ScenarioFailed),
                details: Some(error.to_string()),
            },
            notes: None,
            succeeded: false,
        },
    };

    let completed_at = now_unix_ms()?;
    let report = CommandExecutionReport {
        schema_version: HARNESS_SCHEMA_VERSION.to_owned(),
        metadata: RunMetadata::new(
            "capture",
            &run_id,
            git_sha,
            cwd.display().to_string(),
            PlatformMetadata::current(),
            RunTimestamps { started_at_unix_ms: started_at, completed_at_unix_ms: completed_at },
        ),
        command_name: "capture".to_owned(),
        result: report_result.result,
        artifacts,
        notes: report_result.notes,
    };

    let terminal_report_path = write_json_artifact(&layout, TERMINAL_REPORT_FILENAME, &report)
        .map_err(|error| format!("failed to write capture terminal report: {error}"))?;

    Ok(BootstrapCommandOutcome { terminal_report_path, succeeded: report_result.succeeded })
}

pub fn run_lookdev(args: impl Iterator<Item = String>) -> Result<BootstrapCommandOutcome, String> {
    let options = LookdevCommandOptions::parse(args)?;
    emit_unavailable_report(
        "lookdev",
        options.run_id.unwrap_or_else(|| default_run_id("lookdev", &options.tweak_pack_path)),
        format!(
            "Lookdev sweep is not implemented yet for pack `{}` and camera `{}`.",
            options.tweak_pack_path, options.camera_set
        ),
        format!("Lookdev automation lands in PR-032; requested seed was {}.", options.seed_hex),
        vec![source_artifact("tweak_pack_source", &options.tweak_pack_path, "text/ron")],
    )
}

pub fn run_perf(args: impl Iterator<Item = String>) -> Result<BootstrapCommandOutcome, String> {
    let options = ScenarioCommandOptions::parse("perf", args)?;
    emit_unavailable_report(
        "perf",
        options.run_id.unwrap_or_else(|| default_run_id("perf", &options.scenario_path)),
        format!("Perf check is not implemented yet for scenario `{}`.", options.scenario_path),
        "The stable CLI surface exists now; perf instrumentation lands in PR-036.".to_owned(),
        vec![source_artifact("scenario_source", &options.scenario_path, "text/ron")],
    )
}

fn emit_unavailable_report(
    command_name: &str,
    run_id: String,
    summary: String,
    detail: String,
    mut artifacts: Vec<ArtifactDescriptor>,
) -> Result<BootstrapCommandOutcome, String> {
    let layout = ArtifactLayout::new(command_name, &run_id);
    let started_at = now_unix_ms()?;
    let git_sha = current_git_sha().unwrap_or_else(|_| String::from("<unknown>"));
    let cwd =
        std::env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))?;
    let completed_at = now_unix_ms()?;

    artifacts.insert(
        0,
        ArtifactDescriptor {
            role: "terminal_report".to_owned(),
            path: layout.terminal_report_path_string(),
            media_type: "application/json".to_owned(),
        },
    );

    let report = CommandExecutionReport {
        schema_version: HARNESS_SCHEMA_VERSION.to_owned(),
        metadata: RunMetadata::new(
            command_name,
            &run_id,
            git_sha,
            cwd.display().to_string(),
            PlatformMetadata::current(),
            RunTimestamps {
                started_at_unix_ms: started_at,
                completed_at_unix_ms: completed_at,
            },
        ),
        command_name: command_name.to_owned(),
        result: ResultEnvelope {
            status: HarnessStatus::Failed,
            summary,
            failure_kind: Some(FailureKind::BuildFailed),
            details: Some(detail),
        },
        artifacts,
        notes: Some(vec![
            "Bootstrap command stub only; the CLI surface is intentionally reserved before the owning roadmap task lands.".to_owned(),
        ]),
    };

    let terminal_report_path = write_json_artifact(&layout, TERMINAL_REPORT_FILENAME, &report)
        .map_err(|error| format!("failed to write {command_name} terminal report: {error}"))?;

    Ok(BootstrapCommandOutcome { terminal_report_path, succeeded: false })
}

#[derive(Debug)]
struct ReportBuildResult {
    result: ResultEnvelope,
    notes: Option<Vec<String>>,
    succeeded: bool,
}

#[derive(Debug, Serialize)]
struct CaptureMetadataArtifact {
    adapter: wr_render_api::GraphicsAdapterInfo,
    frame: wr_render_api::CapturedFrameInfo,
}

fn default_run_id(command_name: &str, source_path: &str) -> String {
    format!("{command_name}-{}-{}", artifact_component(source_path), now_unix_ms().unwrap_or(0))
}

fn source_artifact(role: &str, path: &str, media_type: &str) -> ArtifactDescriptor {
    let source_path = Path::new(path);
    let path = if source_path.exists() {
        source_path.to_string_lossy().into_owned()
    } else {
        path.to_owned()
    };

    ArtifactDescriptor { role: role.to_owned(), path, media_type: media_type.to_owned() }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScenarioCommandOptions {
    scenario_path: String,
    run_id: Option<String>,
}

impl ScenarioCommandOptions {
    fn parse(command_name: &str, mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut scenario_path = None;
        let mut run_id = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scenario" => {
                    scenario_path = Some(args.next().ok_or_else(|| {
                        format!("expected a value after --scenario for {command_name}")
                    })?);
                }
                "--run-id" => {
                    run_id = Some(args.next().ok_or_else(|| {
                        format!("expected a value after --run-id for {command_name}")
                    })?);
                }
                value if scenario_path.is_none() => {
                    scenario_path = Some(value.to_owned());
                }
                other => {
                    return Err(format!(
                        "unsupported argument `{other}` for {command_name}; supported flags: --scenario, --run-id"
                    ));
                }
            }
        }

        Ok(Self {
            scenario_path: scenario_path.ok_or_else(|| {
                format!("missing scenario path; pass --scenario <path> to {command_name}")
            })?,
            run_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LookdevCommandOptions {
    tweak_pack_path: String,
    camera_set: String,
    seed_hex: String,
    run_id: Option<String>,
}

impl LookdevCommandOptions {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut tweak_pack_path = None;
        let mut camera_set = None;
        let mut seed_hex = None;
        let mut run_id = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--pack" => {
                    tweak_pack_path = Some(
                        args.next().ok_or_else(|| String::from("expected a value after --pack"))?,
                    );
                }
                "--camera-set" => {
                    camera_set = Some(
                        args.next()
                            .ok_or_else(|| String::from("expected a value after --camera-set"))?,
                    );
                }
                "--seed" => {
                    seed_hex = Some(
                        args.next().ok_or_else(|| String::from("expected a value after --seed"))?,
                    );
                }
                "--run-id" => {
                    run_id = Some(
                        args.next()
                            .ok_or_else(|| String::from("expected a value after --run-id"))?,
                    );
                }
                other => {
                    return Err(format!(
                        "unsupported argument `{other}` for lookdev; supported flags: --pack, --camera-set, --seed, --run-id"
                    ));
                }
            }
        }

        Ok(Self {
            tweak_pack_path: tweak_pack_path
                .ok_or_else(|| String::from("missing tweak pack path; pass --pack <path>"))?,
            camera_set: camera_set
                .ok_or_else(|| String::from("missing camera set; pass --camera-set <name>"))?,
            seed_hex: seed_hex.ok_or_else(|| String::from("missing seed; pass --seed <hex>"))?,
            run_id,
        })
    }
}
