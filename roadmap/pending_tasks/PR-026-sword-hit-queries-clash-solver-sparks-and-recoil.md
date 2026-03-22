# PR-026 - Sword hit queries, clash solver, sparks, and recoil

- Status: pending
- Category: Roadmap Task
- Lane: Gameplay
- Depends on: PR-024, PR-025, PR-009
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-026`

## Primary Crates
- wr_combat
- wr_physics
- wr_vfx

## Scope
Add the contact logic that makes duels legible and satisfying without full rigid-body sword simulation.

## Requirements
- Use swept blade volumes/capsules for hit detection.
- Implement selective sword-on-sword clash handling, parry windows, recoil impulses to controllers, and blade spark events.
- Keep contact resolution bounded and stylized rather than fully emergent.

## Acceptance Criteria
- Sword-to-wraith hits, sword-to-sword clashes, and parry interactions produce distinct events.
- Clashes create visible spark events and push moves into recovery or rebound states.
- No authored animation clips are involved in the resolution path.

## Verification
- Continuous collision tests for swept blade volumes.
- State transition tests for clash/recovery/parry cases.
- Headless duel micro-scenarios with expected event sequences.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
