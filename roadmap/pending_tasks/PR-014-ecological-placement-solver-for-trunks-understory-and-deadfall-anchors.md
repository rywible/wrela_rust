# PR-014 - Ecological placement solver for trunks, understory, and deadfall anchors

- Status: pending
- Category: Roadmap Task
- Lane: World
- Depends on: PR-012
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-014`

## Primary Crates
- wr_world_gen
- wr_world_seed

## Scope
Place major world objects from ecological rules rather than paint-by-hand authoring.

## Requirements
- Use scalar fields plus blue-noise/Poisson-style sampling and competition rules to place tree candidates.
- Bias tree spacing, understory density, and fallen-log anchors from slope, moisture, and canopy opportunity.
- Export placement maps and stats for harness inspection.

## Acceptance Criteria
- Placements are deterministic and stable per seed.
- No major placements intersect forbidden hero-path corridors or impossible slopes.
- Density and spacing targets are configurable and observable in reports.

## Verification
- Property tests for minimum spacing and forbidden-zone compliance.
- Snapshot tests for per-seed density distributions.
- Benchmarks for placement solve time.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
