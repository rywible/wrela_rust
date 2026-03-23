# PR-010 - wgpu backend skeleton, offscreen render path, and frame capture

- Status: pending
- Category: Roadmap Task
- Lane: Render
- Depends on: PR-006, PR-007, PR-009
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-010`

## Primary Crates
- wr_render_api
- wr_render_wgpu
- apps/wr_client
- apps/wr_headless

## Scope
Bring up the GPU backend and prove both on-screen and offscreen rendering work.

## Requirements
- Adopt wgpu as the graphics abstraction and WGSL as the shading language.
- Support swapchain rendering for the client and offscreen texture rendering for headless capture.
- Add frame capture to PNG for deterministic inspection.

## Acceptance Criteria
- Client can clear a window and present frames.
- Headless runner can render to an offscreen target and save a PNG.
- Render device/adapter selection is reported in telemetry.

## Verification
- Shader compilation smoke tests.
- Offscreen capture integration test.
- Image output test that verifies dimensions, color space, and non-empty pixels.

## References
- [wgpu documentation](https://docs.rs/wgpu/latest/wgpu/) - Cross-platform Rust graphics API for Metal/Vulkan/D3D12/OpenGL/WebGPU.
- [image crate docs](https://docs.rs/image/latest/image/) - PNG encode/decode and image processing for captures and metrics.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
