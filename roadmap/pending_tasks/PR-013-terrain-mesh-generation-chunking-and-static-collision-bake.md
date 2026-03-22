# PR-013 - Terrain mesh generation, chunking, and static collision bake

- Status: pending
- Category: Roadmap Task
- Lane: World
- Depends on: PR-012, PR-010, PR-011
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-013`

## Primary Crates
- wr_procgeo
- wr_world_gen
- wr_physics
- wr_render_scene

## Scope
Turn terrain fields into renderable geometry and collision.

## Requirements
- Generate chunked terrain meshes from scalar fields with deterministic triangulation.
- Build static collision for the same terrain and expose sample queries.
- Support debug overlays for normals, tangents, and collision wireframe.

## Acceptance Criteria
- Terrain renders in the client and collides in headless tests.
- Chunk seams are crack-free.
- The same terrain seed produces identical mesh statistics and collider stats.

## Verification
- Mesh topology tests.
- Collision raycast tests.
- Snapshot tests for chunk stats.

## References
- [Rapier docs](https://rapier.rs/docs/) - Physics, collision, queries, snapshotting, optional cross-platform determinism.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
