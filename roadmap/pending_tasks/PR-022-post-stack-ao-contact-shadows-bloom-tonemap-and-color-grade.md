# PR-022 - Post stack: AO, contact shadows, bloom, tonemap, and color grade

- Status: pending
- Category: Roadmap Task
- Lane: Lighting
- Depends on: PR-020, PR-021, PR-010, PR-011
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-022`

## Primary Crates
- wr_render_post
- wr_render_wgpu

## Scope
Finish the stylized AAA outdoor image with restrained post-processing.

## Requirements
- Add ambient occlusion or a near-field occlusion approximation suitable for trunk/root grounding.
- Add screen-space contact shadow support for the sword and wraith grounding.
- Add bloom, filmic tonemapping, color grading, vignette, and exposure controls.

## Acceptance Criteria
- Post stack can be toggled per effect for debugging.
- Canonical before/after captures show clear quality gains without crushing readability.
- Post settings are driven entirely by tweak packs.

## Verification
- Parameter packing tests.
- Offscreen regression captures.
- Histogram summary tests to catch broken exposure/grade configurations.

## References
- [Sébastien Hillaire publications page](https://sebh.github.io/publications/) - Volumetric rendering, sky, atmosphere, and cloud references from the same author.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
