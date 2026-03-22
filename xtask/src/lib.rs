#![forbid(unsafe_code)]

mod verify;

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use wr_core::{CrateBoundary, CrateEntryPoint};
use wr_telemetry::{PlatformMetadata, RunMetadata, RunTimestamps, SeedInfo};
use wr_tools_harness::{
    ArtifactLayout, canonical_noop_test_result_bundle, write_test_result_bundle,
};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("xtask", CrateBoundary::Tooling, false)
}

pub const fn supported_commands() -> &'static [&'static str] {
    &["help", "scaffold-status", "noop-harness-report", "verify"]
}

pub fn run(mut args: impl Iterator<Item = String>) -> i32 {
    match args.next().as_deref() {
        None | Some("help") | Some("--help") | Some("-h") => {
            println!("xtask exposes the repo-standard automation entrypoints.");
            println!("available commands: {}", supported_commands().join(", "));
            0
        }
        Some("scaffold-status") => {
            println!("workspace scaffold and verification stack are present.");
            0
        }
        Some("noop-harness-report") => match emit_noop_harness_report(args) {
            Ok(path) => {
                println!("{}", path.display());
                0
            }
            Err(error) => {
                eprintln!("failed to emit noop harness report: {error}");
                1
            }
        },
        Some("verify") => match verify::run(args) {
            Ok(path) => {
                println!("{}", path.display());
                0
            }
            Err(error) => {
                eprintln!("verification stack failed: {error}");
                1
            }
        },
        Some(command) => {
            eprintln!(
                "unsupported xtask command `{command}` in scaffold phase; implement it in its owning roadmap task"
            );
            1
        }
    }
}

fn emit_noop_harness_report(mut args: impl Iterator<Item = String>) -> Result<PathBuf, String> {
    let mut run_id = String::from("local-noop");
    let mut seed_label = String::from("hero_forest");
    let mut seed_value_hex = String::from("0xDEADBEEF");

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--run-id" => {
                run_id =
                    args.next().ok_or_else(|| String::from("expected a value after --run-id"))?;
            }
            "--seed-label" => {
                seed_label = args
                    .next()
                    .ok_or_else(|| String::from("expected a value after --seed-label"))?;
            }
            "--seed-value" => {
                seed_value_hex = args
                    .next()
                    .ok_or_else(|| String::from("expected a value after --seed-value"))?;
            }
            other => {
                return Err(format!(
                    "unsupported argument `{other}` for noop-harness-report; supported flags: --run-id, --seed-label, --seed-value"
                ));
            }
        }
    }

    let layout = ArtifactLayout::new("noop-harness-report", &run_id);
    let started_at = now_unix_ms()?;
    let git_sha = current_git_sha()?;
    let cwd = std::env::current_dir()
        .map_err(|error| format!("failed to read current dir: {error}"))?
        .display()
        .to_string();
    let completed_at = now_unix_ms()?;

    let metadata = RunMetadata::new(
        "noop-harness-report",
        &run_id,
        git_sha,
        cwd,
        PlatformMetadata::current(),
        RunTimestamps { started_at_unix_ms: started_at, completed_at_unix_ms: completed_at },
    );

    let mut seed = SeedInfo::new(seed_label, seed_value_hex);
    seed.stream = Some(String::from("bootstrap"));

    let bundle =
        canonical_noop_test_result_bundle(metadata, seed, layout.terminal_report_path_string());

    write_test_result_bundle(&layout, &bundle).map_err(|error| error.to_string())
}

fn current_git_sha() -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| format!("failed to invoke git: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }

    let sha = String::from_utf8(output.stdout)
        .map_err(|error| format!("git returned non-utf8 output: {error}"))?;
    let trimmed = sha.trim();

    if trimmed.is_empty() {
        return Err(String::from("git returned an empty sha"));
    }

    Ok(trimmed.to_owned())
}

fn now_unix_ms() -> Result<u64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before unix epoch: {error}"))?;

    u64::try_from(duration.as_millis())
        .map_err(|_| String::from("unix millisecond timestamp overflowed u64"))
}
