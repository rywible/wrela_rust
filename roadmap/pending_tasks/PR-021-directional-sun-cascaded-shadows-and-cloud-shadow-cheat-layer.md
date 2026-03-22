# PR-021 - Directional sun, cascaded shadows, and cloud-shadow cheat layer

- Status: pending
- Category: Roadmap Task
- Lane: Lighting
- Depends on: PR-013, PR-016, PR-017, PR-020
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-021`

## Primary Crates
- wr_render_scene
- wr_render_wgpu
- wr_render_atmo

## Scope
Add the lighting backbone for the forest: sun, shadows, and moving shadow breakup.

## Requirements
- Implement stable cascaded shadow maps for terrain, trunks, and alpha-tested foliage.
- Add a cheap cloud-shadow or canopy-shadow modulation layer to break up lighting and sell scale.
- Support per-pass debug views for each cascade and shadow coverage.

## Acceptance Criteria
- Forest floor and trunks receive believable directional shadows.
- Shadow swimming is bounded and acceptable during player motion.
- Canonical captures show readable contrast in both sunlit and shaded regions.

## Verification
- Cascade split math tests.
- Shadow matrix regression tests.
- Offscreen shadow debug captures for canonical camera positions.

## References
- [GPU Gems 3, Next-Generation SpeedTree Rendering](https://developer.nvidia.com/gpugems/gpugems3/part-i-geometry/chapter-4-next-generation-speedtree-rendering) - Foliage and cascaded-shadow practical reference.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
