# PR-020 - Hillaire atmosphere LUTs and sky rendering

- Status: pending
- Category: Roadmap Task
- Lane: Lighting
- Depends on: PR-010, PR-011, PR-005
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-020`

## Primary Crates
- wr_render_atmo
- wr_render_wgpu

## Scope
Implement the hero sky and atmosphere stack for a fixed late-afternoon setup.

## Requirements
- Implement transmittance, multiscattering, sky-view, and aerial-perspective lookups following Hillaire’s approach.
- Support fixed hero time-of-day first; parameter changes trigger LUT regeneration.
- Expose sun elevation, atmosphere density, ozone, and artistic remap controls through the tweak registry.

## Acceptance Criteria
- Sky renders from ground view with stable horizon coloration and aerial perspective.
- LUTs can be debug-viewed in-engine and exported in headless mode.
- Turning atmosphere off/on is isolated and testable.

## Verification
- CPU-side parameter packing tests.
- Shader/offscreen regression images for canonical sky cameras.
- LUT regeneration smoke tests.

## References
- [A Scalable and Production Ready Sky and Atmosphere Rendering Technique](https://sebh.github.io/publications/egsr2020.pdf) - Primary atmosphere implementation reference.
- [UnrealEngineSkyAtmosphere companion project](https://github.com/sebh/UnrealEngineSkyAtmosphere) - Reference implementation accompanying the Hillaire paper.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
