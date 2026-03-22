# PR-002 - Test runner, reproducibility, and verification stack

- Status: completed
- Category: Roadmap Task
- Lane: Harness
- Depends on: PR-000, PR-001
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-002`

## Primary Crates
- workspace root
- xtask
- wr_telemetry

## Scope
Install the default verification stack for the entire project so every later PR lands with the same tooling.

## Requirements
- Adopt cargo-nextest as the default workspace test runner.
- Wire proptest for math/procedural property tests, insta for snapshots, criterion for performance benches, and tracing for structured logs.
- Add machine-readable JUnit and JSON export through xtask wrappers.

## Acceptance Criteria
- `cargo xtask verify` runs fmt, clippy, nextest, snapshot checks, and the selected benchmark group.
- A failing property test shrinks and stores the minimized repro input.
- A failing snapshot test prints a human-readable diff and stores the changed artifact for review.

## Verification
- Self-test xtask command in CI.
- Sample proptest, criterion, and snapshot tests checked into a demo crate.
- Report generation test proves machine-readable outputs land in reports/.

## References
- [cargo-nextest](https://nexte.st/) - Default test runner; process-per-test is useful for GPU and main-thread sensitive tests.
- [Proptest introduction](https://proptest-rs.github.io/proptest/) - Property-based testing with shrinking.
- [Criterion.rs](https://docs.rs/criterion/latest/criterion/) - Statistics-driven performance regression testing.
- [insta documentation](https://docs.rs/insta/latest/insta/) - Snapshot tests for reports, tweak packs, traces, and structural outputs.
- [tracing documentation](https://docs.rs/tracing/latest/tracing/) - Structured spans/events for telemetry and debugging.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
