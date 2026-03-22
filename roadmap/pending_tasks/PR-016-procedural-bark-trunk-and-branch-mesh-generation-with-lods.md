# PR-016 - Procedural bark, trunk, and branch mesh generation with LODs

- Status: pending
- Category: Roadmap Task
- Lane: World
- Depends on: PR-015, PR-010, PR-011
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-016`

## Primary Crates
- wr_procgeo
- wr_render_scene

## Scope
Convert tree graphs into meshable trunk/branch geometry that still feels authored.

## Requirements
- Generate generalized-cylinder trunk and major branch meshes from the tree graph.
- Add bark ridge displacement and taper-aware UV/procedural material coordinates.
- Produce at least three LOD tiers plus debug normals/tangents.

## Acceptance Criteria
- Meshes are watertight enough for stable shadowing and collision proxies.
- LOD transitions preserve silhouette within the chosen budget.
- A tree batch can be generated and rendered in a forest scene.

## Verification
- Topology tests for degenerate triangles and NaNs.
- LOD consistency tests.
- Benchmarks for generation time and vertex counts.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
