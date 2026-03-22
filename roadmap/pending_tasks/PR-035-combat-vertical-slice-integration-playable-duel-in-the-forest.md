# PR-035 - Combat vertical slice integration: playable duel in the forest

- Status: pending
- Category: Roadmap Task
- Lane: Integration
- Depends on: PR-026, PR-027, PR-028, PR-029, PR-030, PR-031, PR-033, PR-034
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-035`

## Primary Crates
- wr_game
- apps/wr_client
- apps/wr_headless

## Scope
Second full slice: fight a wraith in the actual biome.

## Requirements
- Wire player combat, wraith presentation, AI, cloth, VFX, and duel scenarios into the integrated app.
- Add at least one duel clearing generated from the same biome seed and placement rules.
- Add canonical duel scenarios for both human play and autoplay verification.

## Acceptance Criteria
- A human can launch the client, reach a duel clearing, and fight a wraith end-to-end.
- Autoplay can complete the duel scenario and emit a summary report.
- The duel loop uses no imported meshes or animation clips.

## Verification
- Integrated duel scenarios.
- Replay tests for canonical seeds.
- Capture gallery for combat beats.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
