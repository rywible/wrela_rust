# PR-005 - Live tweak registry and in-engine developer UI shell

- Status: pending
- Category: Roadmap Task
- Lane: Harness
- Depends on: PR-000, PR-002
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-005`

## Primary Crates
- wr_tools_ui
- wr_core

## Scope
Create the parameter/tweak backbone so lookdev and combat tuning are first-class, serialized, and scriptable.

## Requirements
- Add a typed tweak registry with namespaces: world, atmosphere, lighting, foliage, player, combat, wraith, VFX.
- Support load/save of tweak packs and diff-friendly serialized output.
- Create a minimal egui overlay shell with a registry inspector and live edit support.

## Acceptance Criteria
- At runtime, tweaks can be changed live and persisted to a pack file.
- A headless scenario can apply the same tweak pack used in the live client.
- A changed tweak marks the relevant subsystem dirty without requiring a restart.

## Verification
- Serialization tests for tweak packs.
- Registry coverage test to ensure every tweak is discoverable and documented.
- Snapshot test for tweak pack diff formatting.

## References
- [egui documentation](https://docs.rs/egui/latest/egui/) - Immediate-mode dev UI for live tweak panels.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
