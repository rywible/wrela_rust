# PR-033 - Autoplay duel bot and gameplay-loop assertions

- Status: pending
- Category: Roadmap Task
- Lane: Automation
- Depends on: PR-003, PR-026, PR-029
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-033`

## Primary Crates
- wr_ai
- wr_tools_harness
- wr_combat

## Scope
Let the autonomous system play the duel loop and verify the game remains playable.

## Requirements
- Implement a scripted/autoplay pilot that uses the same input action layer as a human player.
- Add scenario assertions for time-to-engage, duel duration range, hit/parry counts, and player survivability thresholds.
- Emit duel summaries and replayable input traces.

## Acceptance Criteria
- Headless duel scenarios can be completed by the autoplay pilot.
- A failed duel assertion produces a replay bundle the developer or agent can rerun locally.
- Autoplay can be toggled between white-box helper mode and pure-input black-box mode.

## Verification
- Replay determinism tests.
- Scenario assertion tests.
- Telemetry summary snapshot tests.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
