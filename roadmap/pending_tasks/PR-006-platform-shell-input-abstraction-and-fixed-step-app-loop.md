# PR-006 - Platform shell, input abstraction, and fixed-step app loop

- Status: pending
- Category: Roadmap Task
- Lane: Core Runtime
- Depends on: PR-000, PR-002
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-006`

## Primary Crates
- wr_platform
- apps/wr_client

## Scope
Create the native windowed app shell, input layer, and deterministic frame/fixed-update loop.

## Requirements
- Use winit to create the window, drive OS events, and unify keyboard/mouse/game input into engine actions.
- Add variable-rate render plus fixed-rate simulation stepping.
- Support windowed and borderless modes, resize handling, and frame pacing diagnostics.

## Acceptance Criteria
- Client window opens and closes cleanly on the Mac dev machine.
- Simulation runs at a stable fixed tick regardless of render framerate.
- Input events are captured through an action layer rather than direct key codes.

## Verification
- Unit tests for input mapping and edge transitions.
- Headless timing tests for fixed-step catch-up behavior.
- Manual smoke checklist for window lifecycle and focus changes.

## References
- [winit documentation](https://docs.rs/crate/winit/latest) - Low-level cross-platform windowing and event loop.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
