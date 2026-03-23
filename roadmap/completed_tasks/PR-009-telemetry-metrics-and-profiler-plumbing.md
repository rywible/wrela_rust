# PR-009 - Telemetry, metrics, and profiler plumbing

- Status: completed
- Category: Roadmap Task
- Lane: Core Runtime
- Depends on: PR-002, PR-006, PR-007
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-009`

## Primary Crates
- wr_telemetry
- wr_core

## Scope
Make every later subsystem observable by default.

## Requirements
- Wrap major systems in tracing spans and counters.
- Add per-frame metrics collection for frame time, sim time, render time, draw counts, entity counts, and memory snapshots.
- Gate Tracy integration behind a non-default feature flag.

## Acceptance Criteria
- A scenario run emits structured traces and a metrics summary file.
- Instrumentation overhead can be disabled in release builds.
- Profiling can be turned on without code changes through features/config.

## Verification
- Metrics schema tests.
- Trace span smoke tests.
- Feature-flag compile tests for Tracy enabled vs disabled.

## References
- [tracing documentation](https://docs.rs/tracing/latest/tracing/) - Structured spans/events for telemetry and debugging.
- [tracy-client documentation](https://docs.rs/tracy-client/latest/tracy_client/) - Optional low-overhead profiler integration.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
