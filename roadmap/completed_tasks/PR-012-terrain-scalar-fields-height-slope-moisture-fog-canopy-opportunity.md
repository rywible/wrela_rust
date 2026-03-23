# PR-012 - Terrain scalar fields: height, slope, moisture, fog, canopy opportunity

- Status: pending
- Category: Roadmap Task
- Lane: World
- Depends on: PR-008, PR-007
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-012`

## Primary Crates
- wr_world_gen
- wr_math

## Scope
Generate the low-frequency ecological fields that the forest uses instead of hand-authored placement.

## Requirements
- Generate a 512m x 512m hero biome scalar field set from seed and config.
- Fields must include at minimum height, slope, drainage/moisture, shade/canopy opportunity, deadfall probability, and hero-path bias.
- Generation must be deterministic and fast enough for headless tests.

## Acceptance Criteria
- Canonical seeds produce field summaries and debug dumps.
- Fields are sampled through a stable API used by later generators.
- Debug visualizations can render each field as an overlay.

## Verification
- Property tests for field bounds and continuity.
- Snapshot tests for field summary stats by seed.
- Benchmarks for field generation throughput.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
