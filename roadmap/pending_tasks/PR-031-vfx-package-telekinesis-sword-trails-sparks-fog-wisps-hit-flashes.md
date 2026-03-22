# PR-031 - VFX package: telekinesis, sword trails, sparks, fog wisps, hit flashes

- Status: pending
- Category: Roadmap Task
- Lane: Gameplay
- Depends on: PR-020, PR-026, PR-027
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-031`

## Primary Crates
- wr_vfx
- wr_render_scene
- wr_render_wgpu

## Scope
Add the non-geometry juice that sells combat and supernatural presence.

## Requirements
- Implement procedural particle/ribbon systems for sword trails, clash sparks, sword hum arcs, and wraith fog wisps.
- All VFX must be generated from math, events, and tweak packs, not imported flipbooks.
- Provide event-driven spawning interfaces shared by combat and AI systems.

## Acceptance Criteria
- Every major combat event has a corresponding VFX hook.
- VFX can be disabled per channel for debugging and performance tests.
- Canonical captures show the sword and wraith reading strongly even in shadow.

## Verification
- Spawn/event wiring tests.
- Deterministic particle-seed tests.
- Capture smoke tests for canonical events.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
