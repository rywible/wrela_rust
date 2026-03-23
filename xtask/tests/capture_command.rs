use std::path::PathBuf;
use std::process::{Command, Output};

use wr_tools_harness::{CommandExecutionReport, HarnessStatus};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("workspace root should exist")
}

fn run_xtask(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(workspace_root())
        .args(args)
        .output()
        .expect("xtask should launch")
}

fn load_report(output: &Output) -> CommandExecutionReport {
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8");
    let report_path = stdout.lines().last().expect("stdout should contain the report path");
    let report_path = workspace_root().join(report_path);
    let report = std::fs::read_to_string(report_path).expect("terminal report should be readable");

    serde_json::from_str(&report).expect("capture terminal report should deserialize")
}

#[test]
fn capture_command_writes_png_and_metadata_artifacts() {
    let output = run_xtask(&[
        "capture",
        "--scenario",
        "scenarios/smoke/startup.ron",
        "--run-id",
        "capture-smoke",
    ]);

    assert!(
        output.status.success(),
        "capture command should succeed: stdout=`{}` stderr=`{}`",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report = load_report(&output);
    assert_eq!(report.result.status, HarnessStatus::Passed);
    assert!(report.artifacts.iter().any(|artifact| artifact.role == "capture_png"));
    assert!(report.artifacts.iter().any(|artifact| artifact.role == "capture_metadata"));

    let png_path = workspace_root()
        .join("reports/harness/capture/capture-smoke/frame.png")
        .canonicalize()
        .expect("capture png should exist");
    let metadata_path = workspace_root()
        .join("reports/harness/capture/capture-smoke/capture_metadata.json")
        .canonicalize()
        .expect("capture metadata should exist");

    let image = image::open(&png_path).expect("capture png should load").into_rgba8();
    assert_eq!(image.width(), wr_headless::DEFAULT_CAPTURE_SIZE.width);
    assert_eq!(image.height(), wr_headless::DEFAULT_CAPTURE_SIZE.height);
    assert!(image.pixels().any(|pixel| pixel.0 != [0, 0, 0, 0]));

    let metadata = std::fs::read_to_string(metadata_path).expect("capture metadata should load");
    assert!(metadata.contains("\"shading_language\": \"wgsl\""));
    assert!(metadata.contains("\"color_space\": \"rgba8_unorm_srgb\""));
}
