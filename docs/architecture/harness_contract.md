# Harness Contract

## Purpose

`PR-001` establishes the first machine-readable harness contract so agents can discover requests, reports, artifacts, and failure modes without scraping ad-hoc stdout.

## Schema version

All bootstrap harness payloads use `wr_harness/v1`.

The v1 contract covers these JSON-schema-backed payloads:

- `ScenarioRequest`
- `CaptureRequest`
- `LookdevSweepRequest`
- `DuelReport`
- `PerformanceReport`
- `TestResultBundle`

## Artifact layout

Harness artifacts live in stable paths rooted at `reports/harness/`.

The bootstrap path contract is:

- run directory: `reports/harness/<command>/<run_id>/`
- terminal bundle: `reports/harness/<command>/<run_id>/terminal_report.json`

Later tasks may add more files inside the same run directory, but they should preserve this root layout so agents can locate terminal results deterministically.

The verification stack now also writes stable per-run helper artifacts under the same directory, including:

- `verify_steps.json`
- `trace.jsonl`
- `nextest-junit.xml`
- copied Criterion estimate JSON files when benchmarks run

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

This command is intentionally narrow. Scenario execution, capture, lookdev, replay, and perf surfaces land in later roadmap tasks.

## Verification command

`cargo xtask verify` is now the repo-standard wrapper for:

- bootstrap process contract validation,
- `cargo fmt --check`,
- `cargo clippy --workspace --all-targets -- -D warnings`,
- `cargo nextest run --workspace --profile ci`,
- the selected Criterion benchmark group.

Its terminal bundle still lands at `reports/harness/verify/<run_id>/terminal_report.json`, and the run directory includes machine-readable step records plus the copied JUnit report so agents and CI do not need to scrape ad-hoc stdout.
