# PR-024 - Movement and kinematic physics integration

- Status: pending
- Category: Roadmap Task
- Lane: Gameplay
- Depends on: PR-013, PR-023
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-024`

## Primary Crates
- wr_physics
- wr_actor_player

## Scope
Make navigation and dueling locomotion reliable on procedural terrain.

## Requirements
- Implement grounded movement, jump, air control, dash, slope handling, and simple step-up behavior.
- Use Rapier for terrain queries, broadphase, and character/world collision, but keep the controller game-feel driven.
- Expose movement tuning through tweak packs.

## Acceptance Criteria
- Player can traverse terrain without jitter or tunneling in canonical maps.
- Dash respects collision and recovery rules.
- Jump and landing state transitions are deterministic under fixed-step simulation.

## Verification
- Slope and grounding tests.
- Dash collision tests.
- Property tests for controller invariants under repeated step sequences.

## References
- [Rapier docs](https://rapier.rs/docs/) - Physics, collision, queries, snapshotting, optional cross-platform determinism.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
