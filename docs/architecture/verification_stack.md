# Verification Stack

## Purpose

`PR-002` turns `cargo xtask verify` into the default repo-wide verification wrapper so every later task lands through the same machine-readable surface.

## Tooling choices

- `cargo-nextest` is the default workspace test runner.
- `insta` provides snapshot assertions and reviewable `.snap.new` artifacts.
- `proptest` provides shrinking plus persisted regression inputs.
- `criterion` provides the selected benchmark group for the current workspace phase.
- `tracing` writes structured verification events to `trace.jsonl`.

## Repo config

- `.config/nextest.toml` defines the `ci` profile and JUnit export path.
- `.config/insta.yaml` forces diff output and makes nextest the default snapshot runner.
- `wr_telemetry` carries the demo snapshot, property, and benchmark coverage that proves the stack is wired end to end.

## Command contract

`cargo xtask verify [--run-id <id>]` runs:

1. `python3 docs/process/validate_process_contract.py`
2. `cargo fmt --check`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo nextest list --workspace --profile ci --message-format json`
5. `cargo nextest run --workspace --profile ci --failure-output immediate-final --success-output never --message-format libtest-json-plus`
6. `cargo bench -p wr_telemetry --bench artifact_component -- --noplot`

The nextest run sets `INSTA_OUTPUT=diff` and `INSTA_UPDATE=new` so failing snapshot assertions produce readable diffs and `.snap.new` review artifacts even in CI-like environments.

## Artifacts

Every verify run writes to `reports/harness/verify/<run_id>/`.

Important artifacts:

- `terminal_report.json`: terminal `TestResultBundle`
- `verify_steps.json`: machine-readable step-level command results
- `trace.jsonl`: structured tracing events for the verify orchestration
- `nextest-list.stdout.jsonl`: test inventory export
- `nextest.events.jsonl`: nextest run event stream
- `nextest-junit.xml`: copied JUnit report from the nextest `ci` profile
- `criterion/**/estimates.json`: copied benchmark estimate artifacts

The terminal bundle remains the source of truth for pass/fail, while the sibling artifacts provide drill-down without scraping console logs.
