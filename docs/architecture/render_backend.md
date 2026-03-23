# Render Backend Bootstrap

PR-010 introduces the first concrete GPU backend for Wrela v0.

## Current contract

- `wr_render_api` owns the stable data contract for offscreen render requests, capture metadata, and adapter reporting.
- `wr_render_wgpu` owns `wgpu` device/bootstrap, WGSL shader compilation, offscreen PNG capture, and the minimal surface presenter used by the client shell.
- `wr_platform` keeps ownership of the window loop and input shell, but now delegates frame presentation to the `wgpu` surface renderer during redraw events.
- `xtask capture` is the harness-facing proof path for offscreen rendering and emits stable PNG + JSON artifacts under `reports/harness/capture/<run_id>/`.

## Intentional bootstrap limitation

Scene extraction is not available yet, so the offscreen capture path renders a deterministic seed-derived clear color rather than live gameplay geometry.

This keeps the command surface, PNG output, shader pipeline, adapter selection, and artifact contract stable now, while leaving scene extraction to PR-011.
