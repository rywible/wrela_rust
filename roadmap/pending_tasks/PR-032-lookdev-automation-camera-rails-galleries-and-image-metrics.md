# PR-032 - Lookdev automation: camera rails, galleries, and image metrics

- Status: pending
- Category: Roadmap Task
- Lane: Automation
- Depends on: PR-004, PR-005, PR-010
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-032`

## Primary Crates
- wr_tools_harness
- wr_tools_ui
- wr_render_post

## Scope
Give the agent the ability to iterate on the look of the world without a human in the loop.

## Requirements
- Add named camera rails and canonical still cameras stored as scenario assets.
- Add a lookdev sweep command that applies tweak packs, captures frames, and writes a gallery plus summary metrics.
- Implement permissively licensed image metrics in-house on top of the image crate: histogram deltas, edge density, contrast bands, and a simple SSIM-style metric if needed.

## Acceptance Criteria
- A single command can generate a lookdev gallery for a seed and tweak pack.
- Metrics are emitted alongside captured frames and compared to baselines.
- Agents can request captures without opening a visible window.

## Verification
- Gallery manifest tests.
- Metric math unit tests.
- Integration test generating a small contact sheet from offscreen frames.

## References
- [image crate docs](https://docs.rs/image/latest/image/) - PNG encode/decode and image processing for captures and metrics.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
