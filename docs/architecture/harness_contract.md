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
