# PR-018 - Ground cover, deadfall, roots, and set-dressing generators

- Status: pending
- Category: Roadmap Task
- Lane: World
- Depends on: PR-012, PR-014, PR-016
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-018`

## Primary Crates
- wr_world_gen
- wr_procgeo
- wr_render_scene

## Scope
Fill the forest floor with generated structure so the biome feels authored even though it is not.

## Requirements
- Generate ferns, low brush, roots, fallen trunks, stumps, and rock/debris forms from seed and terrain fields.
- Keep hero traversal readability by respecting movement corridors and combat clearings.
- Expose density controls and debug overlays.

## Acceptance Criteria
- The forest floor reads as rich, layered, and traversable in canonical cameras.
- No deadfall or roots create unavoidable collision traps around duel arenas.
- All generated props derive from math/procedural geometry paths, not imported assets.

## Verification
- Placement validity tests.
- Collision clearance tests around spawn zones.
- Visual smoke tests for canonical camera rails.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
