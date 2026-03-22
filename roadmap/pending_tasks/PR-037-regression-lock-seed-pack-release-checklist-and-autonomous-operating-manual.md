# PR-037 - Regression lock, seed pack, release checklist, and autonomous operating manual

- Status: pending
- Category: Roadmap Task
- Lane: Release
- Depends on: PR-036, PR-035, PR-032, PR-033
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-037`

## Primary Crates
- docs
- wr_tools_harness
- apps/wr_client
- apps/wr_headless
- apps/wr_agentd

## Scope
Freeze the v0 slice into something an autonomous agent can keep evolving without wrecking it.

## Requirements
- Check in canonical seed packs, tweak packs, camera packs, duel scenarios, and replay bundles.
- Write the agent operating manual: allowed commands, expected artifacts, escalation path on failures, and how to cut new baselines.
- Add a release checklist and regression matrix.

## Acceptance Criteria
- A clean machine can build the project, run verify, generate the world, produce lookdev captures, and complete the duel smoke scenario by following docs only.
- Baseline artifacts exist for forest traversal and duel scenarios.
- The agent manual explains how to add new PRs without violating crate ownership and integration rules.

## Verification
- Fresh-clone build-and-verify test.
- Artifact presence tests for baseline packs.
- Docs smoke check through scripted commands.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
