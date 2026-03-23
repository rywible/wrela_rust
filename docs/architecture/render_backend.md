# Render Backend Bootstrap

PR-010 introduces the first concrete GPU backend for Wrela v0.

## Current contract

- `wr_render_api` owns the stable data contract for offscreen render requests, capture metadata, and adapter reporting.
- `wr_render_api` now also owns the immutable extracted-scene payload, render graph resource/pass descriptors, and shader/pipeline asset descriptors that later passes extend.
- `wr_render_wgpu` owns `wgpu` device/bootstrap, WGSL shader compilation, offscreen PNG capture, and the minimal surface presenter used by the client shell.
- `wr_render_wgpu` validates render-graph dependencies against registered shader/pipeline assets and executes the graph through backend pass factories.
- `wr_platform` keeps ownership of the window loop and input shell, but now delegates frame presentation to the `wgpu` surface renderer during redraw events.
- `xtask capture` is the harness-facing proof path for offscreen rendering and emits stable PNG + JSON artifacts under `reports/harness/capture/<run_id>/`.

## Scene extraction contract

- Gameplay code extracts renderable data into `ExtractedRenderScene`, which contains owned immutable primitives and can outlive the ECS borrows used to build it.
- Render passes express ordering through named dependencies plus read/write resource edges.
- The current proof path is a debug triangle extracted from ECS and rendered through the graph. Later render passes should extend the same registry/graph surface instead of bypassing it.

## Intentional limitation

The graph currently targets a single external color attachment and one debug-geometry pipeline. This is deliberate: PR-011 establishes the validation and extraction seam so later render tasks can add atmosphere, shadows, and post passes without changing the core contract.
