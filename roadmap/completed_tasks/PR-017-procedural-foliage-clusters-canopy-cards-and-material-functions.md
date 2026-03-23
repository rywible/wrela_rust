# PR-017 - Procedural foliage clusters, canopy cards, and material functions

- Status: completed
- Category: Roadmap Task
- Lane: World
- Depends on: PR-015, PR-016, PR-010
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-017`

## Primary Crates
- wr_procgeo
- wr_render_scene
- wr_render_wgpu

## Scope
Create the redwood canopy and branch-tip masses without preauthored textures or meshes.

## Requirements
- Generate procedural foliage cluster geometry from tree branch tips and canopy envelopes.
- Use procedural alpha masks, gradients, and normals rather than painted textures.
- Support instancing and billboarding only where it is visually safe.

## Acceptance Criteria
- Canopy density is controllable and readable from hero-ground views.
- Foliage materials compile without external textures except allowed small LUTs/noise tables.
- Tree batches remain within the vertex and draw-call targets defined in docs/perf-budget.md.

## Verification
- Material parameter packing tests.
- Snapshot tests for canopy statistics per tree.
- Offscreen visual smoke tests for near and far canopy views.

## References
- [GPU Gems 3, Next-Generation SpeedTree Rendering](https://developer.nvidia.com/gpugems/gpugems3/part-i-geometry/chapter-4-next-generation-speedtree-rendering) - Foliage and cascaded-shadow practical reference.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
