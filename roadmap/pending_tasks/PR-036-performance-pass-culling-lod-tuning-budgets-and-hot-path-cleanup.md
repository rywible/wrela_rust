# PR-036 - Performance pass: culling, LOD tuning, budgets, and hot-path cleanup

- Status: pending
- Category: Roadmap Task
- Lane: Optimization
- Depends on: PR-034, PR-035
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-036`

## Primary Crates
- wr_render_scene
- wr_render_wgpu
- wr_world_gen
- wr_procgeo
- wr_telemetry

## Scope
Make the vertical slice reliably hit the target envelope on the dev machine.

## Requirements
- Tune draw call counts, instance submission, mesh LOD thresholds, shadow caster selection, and update frequencies.
- Add frustum/distance culling and any cheap CPU-side visibility filters needed for the static world.
- Document frame budgets and fail the performance gate when budgets regress.

## Acceptance Criteria
- Canonical traversal and duel scenarios meet the 1080p60 target on the Mac dev machine in release mode, or the gap is quantified with a documented cut plan.
- Performance reports include per-pass breakdowns.
- Visual quality regressions from optimization work are caught by capture tests.

## Verification
- Criterion or custom frame-time benchmarks.
- Integrated performance gate scenarios.
- Visual regression checks on optimized vs baseline captures.

## References
- [Criterion.rs](https://docs.rs/criterion/latest/criterion/) - Statistics-driven performance regression testing.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
