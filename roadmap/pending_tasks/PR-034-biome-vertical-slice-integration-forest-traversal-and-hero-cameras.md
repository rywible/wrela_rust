# PR-034 - Biome vertical slice integration: forest traversal and hero cameras

- Status: pending
- Category: Roadmap Task
- Lane: Integration
- Depends on: PR-019, PR-020, PR-021, PR-022, PR-023, PR-024, PR-032
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-034`

## Primary Crates
- wr_game
- apps/wr_client
- apps/wr_headless

## Scope
First full slice: walk through the generated biome with the hero lighting stack online.

## Requirements
- Wire worldgen, terrain, forest rendering, atmosphere, lighting, post, movement, and dev UI into the game app.
- Spawn canonical hero cameras and traversal paths for lookdev and performance captures.
- Add a packaged smoke scenario for ‘spawn, traverse, dash, jump, look around, capture gallery’.

## Acceptance Criteria
- The user can launch the client and walk a generated redwood grove with hero lighting.
- The lookdev sweep works on the integrated slice.
- Performance and image reports are emitted from the same integrated build.

## Verification
- Traversal smoke scenarios.
- Integrated offscreen capture regression set.
- Frame-time benchmark over a canonical traversal path.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
