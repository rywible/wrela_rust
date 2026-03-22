# PR-030 - Enemy move composers and flashy move-set families

- Status: pending
- Category: Roadmap Task
- Lane: Gameplay
- Depends on: PR-025, PR-029, PR-028
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-030`

## Primary Crates
- wr_ai
- wr_combat
- wr_actor_wraith

## Scope
Give the wraith a more theatrical move vocabulary than the player.

## Requirements
- Implement at least five enemy move families: quick cut, overhead heavy, dash lunge, sweeping multi-cut, and feint-to-counter.
- Move families are still parameterized trajectories, not clips.
- Each move family advertises telegraph/readability windows for gameplay testing.

## Acceptance Criteria
- Canonical duel scenarios demonstrate all enemy move families.
- Enemy moves are visually distinct and identifiable in telemetry traces.
- Flashiness does not break parry/recovery rules.

## Verification
- Move-family registry tests.
- Scenario coverage tests ensuring each move family is exercised.
- Visual capture set for enemy move gallery.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
