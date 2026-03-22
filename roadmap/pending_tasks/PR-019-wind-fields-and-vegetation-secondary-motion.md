# PR-019 - Wind fields and vegetation secondary motion

- Status: pending
- Category: Roadmap Task
- Lane: World
- Depends on: PR-017, PR-005
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-019`

## Primary Crates
- wr_world_gen
- wr_render_scene
- wr_render_wgpu

## Scope
Add believable motion to the forest without turning it into a full simulation project.

## Requirements
- Define low-frequency global wind plus localized gust noise fields.
- Drive trunk sway, branch flex, and canopy shimmer from analytical functions and per-instance parameters.
- Keep all motion deterministic under a given time origin for capture tests.

## Acceptance Criteria
- Static screenshots can opt to freeze wind; live runs show layered motion with clear frequency separation.
- Wind response is parameterized by tree size and canopy mass.
- Motion remains stable under pause/resume and step-frame tooling.

## Verification
- Parameter serialization tests.
- Time-scrub determinism tests.
- Offscreen captures at fixed times verifying repeatable transforms.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
