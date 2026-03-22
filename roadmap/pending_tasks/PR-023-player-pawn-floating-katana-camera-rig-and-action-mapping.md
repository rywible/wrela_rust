# PR-023 - Player pawn: floating katana camera rig and action mapping

- Status: pending
- Category: Roadmap Task
- Lane: Gameplay
- Depends on: PR-006, PR-007, PR-005
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-023`

## Primary Crates
- wr_actor_player
- wr_game

## Scope
Implement the sword-as-character first-person presentation and input verbs.

## Requirements
- Create the camera rig, sword anchor rig, and action map for strafe, jump, dash, light attack, heavy attack, and parry.
- No human body, arms, or sleeves in v0; only sword and VFX presentation.
- Keep sword presentation decoupled from combat solve so lookdev can iterate independently.

## Acceptance Criteria
- Player can move in a blank test map with sword visible and responsive.
- Action buffering exists for combat verbs.
- Camera and sword bob/sway are tweakable and can be disabled for tests.

## Verification
- Input-to-action tests.
- Camera transform tests.
- Headless action buffer tests.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
