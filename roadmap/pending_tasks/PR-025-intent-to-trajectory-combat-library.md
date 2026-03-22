# PR-025 - Intent-to-trajectory combat library

- Status: pending
- Category: Roadmap Task
- Lane: Gameplay
- Depends on: PR-023, PR-005, PR-008
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-025`

## Primary Crates
- wr_combat
- wr_math

## Scope
Create the authored-feeling mathematical move families that replace traditional animation clips.

## Requirements
- Map verbs to parameterized trajectory families, not raw hand-authored poses.
- Support guard, anticipation, strike, follow-through, recovery, and cancel windows.
- Use spring/PD-style controllers and spline/path parameterization to keep motion readable and sick-looking.

## Acceptance Criteria
- Player sword can execute light, heavy, dash slash, and parry trajectories in isolation.
- Trajectory families are deterministic given verb + seed + tuning pack.
- Moves expose enough parameters for lookdev without requiring code edits.

## Verification
- Trajectory sampling tests.
- Property tests for continuity, bounded angular velocity, and recovery completion.
- Golden trajectory snapshot tests for canonical moves.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
