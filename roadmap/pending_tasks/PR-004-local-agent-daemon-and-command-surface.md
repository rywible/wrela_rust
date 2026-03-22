# PR-004 - Local agent daemon and command surface

- Status: pending
- Category: Roadmap Task
- Lane: Harness
- Depends on: PR-001, PR-003
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-004`

## Primary Crates
- wr_tools_harness
- apps/wr_agentd

## Scope
Wrap build/test/run/capture workflows in a thin local service so autonomous agents can interact through a stable API instead of shell heuristics.

## Requirements
- Add a local-only daemon exposing commands for verify, run_scenario, capture_frames, lookdev_sweep, and perf_check.
- The daemon should spawn subprocesses, stream logs, and return artifact locations.
- CLI remains source of truth; daemon is only a wrapper around shared library code.

## Acceptance Criteria
- An HTTP request can launch a scenario and return a report descriptor with artifact paths.
- The daemon can manage concurrent jobs and preserve per-job output directories.
- The same command works from CLI and daemon, with identical report payloads.

## Verification
- API contract tests for all endpoints.
- Subprocess supervision tests.
- Concurrency test with at least two simultaneous no-op jobs.

## References
- [Tokio homepage](https://tokio.rs/) - Async runtime for the local daemon and harness services.
- [axum documentation](https://docs.rs/axum/latest/axum/) - HTTP API for the local-only agent daemon.
- [tracing documentation](https://docs.rs/tracing/latest/tracing/) - Structured spans/events for telemetry and debugging.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
