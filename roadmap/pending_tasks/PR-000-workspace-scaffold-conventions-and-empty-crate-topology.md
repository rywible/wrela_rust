# PR-000 - Workspace scaffold, conventions, and empty crate topology

- Status: pending
- Category: Roadmap Task
- Lane: Foundation
- Depends on: None
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-000`

## Primary Crates
- workspace root
- xtask
- all crate skeletons

## Scope
Pre-create the full Cargo workspace, empty crate boundaries, CI stubs, lint config, docs/ADR structure, and placeholder plugin interfaces so later PRs mostly stay inside one crate.

## Requirements
- Create the full workspace and crate layout, including empty public APIs for every planned subsystem.
- Add rust-toolchain, cargo aliases, fmt/clippy policy, feature-flag conventions, and module scaffolding.
- Add docs/adr, docs/architecture, docs/tuning, scenarios/, reports/, generated-cache/ conventions.
- Define the merge policy: subsystem crates only; wr_game and app crates are integration-only.

## Acceptance Criteria
- `cargo check --workspace` passes with all placeholder crates.
- Every future PR in this plan can land by modifying at most one subsystem crate plus tests, until integration milestones.
- ADR-000 documents scope cuts for v0: one biome, one hero time-of-day, one enemy archetype, one weapon, one non-streaming map.

## Verification
- Workspace compiles on the target Mac dev machine.
- CI smoke job runs fmt, clippy, and cargo check.
- One compile-only test validates that every crate exposes a plugin/init entrypoint.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
