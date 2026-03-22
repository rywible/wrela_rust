# PR-003 - Headless scenario runner skeleton

- Status: pending
- Category: Roadmap Task
- Lane: Harness
- Depends on: PR-001, PR-002
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-003`

## Primary Crates
- wr_tools_harness
- wr_ecs
- wr_world_seed
- wr_game
- apps/wr_headless

## Scope
Create the deterministic, no-window execution path that drives all autonomous validation.

## Requirements
- Add a headless binary that can load a scenario, simulate N fixed steps, and emit JSON reports.
- Scenario files must include seed, simulation rate, spawned actors, scripted inputs, and assertions.
- The runner must fail fast on assertion errors and always write a terminal report.

## Acceptance Criteria
- `wr_headless --scenario scenarios/smoke.ron` exits with code 0 and writes a valid report.
- Simulation can run without any GPU/window dependencies.
- The same scenario and seed produces identical report data on repeated runs on the same machine.

## Verification
- Integration tests for smoke scenarios.
- Determinism regression test: same scenario twice, identical report hash.
- Crash-resilience test: forced assertion failure still emits a terminal report file.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
