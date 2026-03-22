# PR-029 - Wraith duel AI base: spacing, telegraphs, and reaction model

- Status: pending
- Category: Roadmap Task
- Lane: Gameplay
- Depends on: PR-026, PR-027, PR-024
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-029`

## Primary Crates
- wr_ai
- wr_actor_wraith
- wr_combat

## Scope
Get the enemy to duel, not just animate.

## Requirements
- Implement a deterministic duel planner with spacing control, threat evaluation, parry/recovery awareness, and telegraph timing.
- AI decisions should operate on combat state and visibility, not animation clip timing.
- Expose behavior tuning packs for aggression, patience, feint frequency, and dash probability.

## Acceptance Criteria
- A single wraith can approach, attack, retreat, and punish predictable player behavior.
- AI state transitions are reported in telemetry and scenario logs.
- Same seed + same input sequence => same AI decision trace on the same machine.

## Verification
- Decision trace snapshot tests.
- Scenario tests for spacing and telegraph timing.
- Property tests for valid state transitions.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
