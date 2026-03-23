# Harness Contract

## Purpose

`PR-001` establishes the first machine-readable harness contract so agents can discover requests, reports, artifacts, and failure modes without scraping ad-hoc stdout.

## Schema version

All bootstrap harness payloads use `wr_harness/v1`.

The v1 contract covers these JSON-schema-backed payloads:

- `ScenarioRequest`
- `ScenarioExecutionReport`
- `CaptureRequest`
- `LookdevSweepRequest`
- `DuelReport`
- `PerformanceReport`
- `CommandExecutionReport`
- `DaemonLaunchRequest`
- `DaemonJobSnapshot`
- `TestResultBundle`

## Artifact layout

Harness artifacts live in stable paths rooted at `reports/harness/`.

The bootstrap path contract is:

- run directory: `reports/harness/<command>/<run_id>/`
- terminal bundle: `reports/harness/<command>/<run_id>/terminal_report.json`

Later tasks may add more files inside the same run directory, but they should preserve this root layout so agents can locate terminal results deterministically.

`PR-003` makes `run-scenario` real. Its terminal report is now a `ScenarioExecutionReport`, and canonical scenarios are authored in RON under `scenarios/`.

The verification stack now also writes stable per-run helper artifacts under the same directory, including:

- `verify_steps.json`
- `trace.jsonl`
- `nextest-junit.xml`
- copied Criterion estimate JSON files when benchmarks run

`PR-004` adds a daemon-side job wrapper. Each daemon job also gets a stable local directory under `reports/harness/daemon/<job_id>/` with streamed `stdout.log` and `stderr.log` artifacts, while the underlying command still writes its normal terminal report under `reports/harness/<command>/<run_id>/terminal_report.json`.

## Failure taxonomy

The v1 failure taxonomy is:

- `build_failed`
- `test_failed`
- `scenario_failed`
- `perf_regressed`
- `visual_regressed`
- `runtime_crash`

## Bootstrap command

`cargo xtask noop-harness-report` emits a valid `TestResultBundle` with:

- git SHA
- run timestamps
- platform metadata
- working directory
- seed label and value
- stable artifact paths

This command is intentionally narrow. Replay still lands in a later roadmap task.

## Headless scenario command

`wr_headless --scenario <path>` and `cargo xtask run-scenario <path>` now load a RON scenario, execute a deterministic fixed-step run, and emit `reports/harness/run-scenario/<run_id>/terminal_report.json`.

The report includes:

- scenario identity,
- seed metadata,
- step-count metrics,
- per-assertion results,
- stable artifact paths,
- a deterministic hash that ignores timestamp-only metadata churn.

## Verification command

`cargo xtask verify` is now the repo-standard wrapper for:

- bootstrap process contract validation,
- `cargo fmt --check`,
- `cargo clippy --workspace --all-targets -- -D warnings`,
- `cargo nextest run --workspace --profile ci`,
- the selected Criterion benchmark group.

Its terminal bundle still lands at `reports/harness/verify/<run_id>/terminal_report.json`, and the run directory includes machine-readable step records plus the copied JUnit report so agents and CI do not need to scrape ad-hoc stdout.

## Reserved bootstrap command surfaces

`PR-004` reserves the remaining daemon-facing CLI commands so agents can target stable names before their full implementations land:

- `cargo xtask capture --scenario <path> [--run-id <id>]`
- `cargo xtask lookdev --pack <path> --camera-set <name> --seed <hex> [--run-id <id>]`
- `cargo xtask perf --scenario <path> [--run-id <id>]`
- `cargo xtask daemon [--bind <addr>] [--workspace-root <path>]`

Until the owning runtime tasks land, `capture`, `lookdev`, and `perf` emit a `CommandExecutionReport` with a clear bootstrap-unavailable failure instead of silently missing the command surface.

## Local agent daemon

`wr_agentd` and `cargo xtask daemon` now expose a local-only HTTP surface that accepts a `DaemonLaunchRequest`, spawns the matching CLI command as a subprocess, and returns a `DaemonJobSnapshot`.

The daemon contract guarantees that:

- job launch returns a stable job ID plus predicted artifact paths,
- job status polling returns queued/running/succeeded/failed state transitions,
- daemon stdout/stderr are streamed into stable local artifacts while the subprocess runs,
- the underlying CLI command remains the source of truth for the command payload written to `reports/harness/<command>/<run_id>/terminal_report.json`.

This keeps the agent-facing API stable without inventing a second execution path that could drift away from the CLI contract.
