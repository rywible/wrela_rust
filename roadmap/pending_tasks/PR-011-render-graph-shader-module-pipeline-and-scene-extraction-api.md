# PR-011 - Render graph, shader module pipeline, and scene extraction API

- Status: pending
- Category: Roadmap Task
- Lane: Render
- Depends on: PR-010, PR-007
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-011`

## Primary Crates
- wr_render_api
- wr_render_wgpu
- wr_game

## Scope
Define how gameplay data becomes renderable data without tangling sim and rendering code.

## Requirements
- Implement a small explicit render graph with named passes and resource edges.
- Add an extract stage that copies immutable render-ready data out of gameplay ECS into render structs.
- Define pipeline/shader asset boundaries so later PRs add passes without editing core code.

## Acceptance Criteria
- A debug triangle or cube can be extracted from ECS and rendered through the graph.
- Render passes can declare dependencies and be validated.
- Scene extraction is frame-safe and does not retain mutable gameplay borrows.

## Verification
- Render graph validation tests.
- Extraction contract tests.
- Compile-time feature tests for pass registration.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
