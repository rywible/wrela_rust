# PR-008 - Deterministic seed graph, RNG, and config packs

- Status: pending
- Category: Roadmap Task
- Lane: Core Runtime
- Depends on: PR-000, PR-002
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-008`

## Primary Crates
- wr_world_seed
- wr_core
- wr_math

## Scope
Create the deterministic seed and config story that every procedural subsystem uses.

## Requirements
- Define a root world seed and hierarchical sub-seeds for terrain, ecology, trees, wraiths, combat scenarios, and VFX.
- Never allow deterministic codepaths to depend on hash map iteration order or global RNG state.
- Support named config packs that override defaults but keep the same seed topology.

## Acceptance Criteria
- Same seed + same config pack => identical generated stats on repeated runs.
- Changing one sub-seed only changes the owning subsystem outputs.
- The report bundle includes root seed and sub-seed derivation info.

## Verification
- Property tests for seed derivation uniqueness and stability.
- Snapshot tests for generated seed trees.
- Cross-run determinism tests.

## References
- None.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
