# PR-007 - ECS world, schedules, and plugin composition model

- Status: completed
- Category: Roadmap Task
- Lane: Core Runtime
- Depends on: PR-000, PR-006
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-007`

## Primary Crates
- wr_ecs
- wr_game

## Scope
Stand up the gameplay world and the scheduling model that all systems plug into.

## Requirements
- Use standalone Bevy ECS for entities, resources, systems, and schedules.
- Define explicit schedules: Startup, FixedPrePhysics, FixedPhysics, FixedGameplay, FixedPostGameplay, Extract, RenderPrep, Shutdown.
- Provide a plugin registration pattern so subsystem crates can export systems without touching integration code.

## Acceptance Criteria
- A sample plugin can register systems and resources into the game without global mutable registries.
- Schedule ordering and ambiguity checks are enabled for debug builds.
- System sets exist for worldgen, combat, AI, rendering extraction, and tooling.

## Verification
- Schedule ordering tests.
- System parallelism/ambiguity smoke tests.
- Entity/resource lifecycle tests.

## References
- [bevy_ecs documentation](https://docs.rs/crate/bevy_ecs/latest) - Standalone ECS with schedules, systems, resources, and parallel execution.

## Completion Workflow
- When this task is completed and its implementation PR is opened, move this file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
