# PR-027 - Wraith body graph, silhouette mesh, and presentation rig

- Status: pending
- Category: Roadmap Task
- Lane: Gameplay
- Depends on: PR-007, PR-016, PR-005
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-027`

## Primary Crates
- wr_actor_wraith
- wr_procgeo
- wr_render_scene

## Scope
Create the enemy body presentation that dodges the uncanny valley and stays procedural.

## Requirements
- Build a non-human, silhouette-first wraith rig: core body graph, head/torso suggestion, blade anchor, cloth pins, and emissive/fog anchors.
- Generate mesh/shell geometry procedurally from that graph.
- Support readable enemy posing from a small set of procedural pose families.

## Acceptance Criteria
- Wraith can be instantiated and rendered with a stable silhouette from multiple combat distances.
- No faces, fingers, or human-detail requirements exist in v0.
- Presentation parameters are tweakable and seed-driven.

## Verification
- Mesh validity tests.
- Pose family snapshot tests.
- Canonical camera smoke captures.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
