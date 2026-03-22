# PR-028 - Wraith cloth and ribbon simulation with XPBD

- Status: pending
- Category: Roadmap Task
- Lane: Gameplay
- Depends on: PR-027, PR-024
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-028`

## Primary Crates
- wr_actor_wraith
- wr_math
- wr_render_scene

## Scope
Add low-cost, expressive trailing cloth to the wraith silhouette.

## Requirements
- Implement low-resolution strip/ribbon cloth using XPBD or equivalent compliant constraints.
- Pin cloth to the wraith rig and drive secondary motion from root/body acceleration plus wind.
- Add collision only where it materially improves the silhouette; avoid full cloth-world interaction in v0.

## Acceptance Criteria
- Cloth motion is stable under aggressive movement and dash attacks.
- Solver parameters are exposed and hot-tweakable.
- The cloth system can be fully disabled for performance comparisons.

## Verification
- Constraint stability tests.
- Pause/resume/time-step consistency tests.
- Visual smoke tests for canonical attack motions.

## References
- [XPBD: Position-Based Simulation of Compliant Constrained Dynamics](https://matthias-research.github.io/pages/publications/XPBD.pdf) - Primary cloth/ribbon constraint solver reference.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
