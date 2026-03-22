# PR-001 - Agent harness contract and artifact schema

- Status: pending
- Category: Roadmap Task
- Lane: Harness
- Depends on: PR-000
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-001`

## Primary Crates
- wr_tools_harness
- wr_telemetry

## Scope
Define the machine-readable contract the autonomous agent will use: commands, reports, artifacts, result envelopes, and failure modes.

## Requirements
- Define JSON schemas for scenario request, capture request, lookdev sweep request, duel report, performance report, and test result bundle.
- Define stable artifact paths and naming conventions so agents can discover outputs without scraping logs.
- Define the error taxonomy: build_failed, test_failed, scenario_failed, perf_regressed, visual_regressed, runtime_crash.

## Acceptance Criteria
- Schemas are versioned and validated during tests.
- A no-op harness command can emit a valid report bundle with metadata, timestamps, git SHA, and seed info.
- At least one end-to-end golden JSON report is checked in as a reference artifact.

## Verification
- Schema round-trip serialization tests.
- Snapshot tests for canonical report payloads.
- Property tests for backward-compatible report parsing when optional fields are added.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
