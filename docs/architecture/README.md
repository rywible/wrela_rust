# Architecture Notes

This directory holds architecture documents that explain how the current repo shape maps to the planned runtime.

- `harness_contract.md` explains the machine-readable harness request/report contract.
- `seed_graph.md` explains the deterministic root seed topology, named seed config packs, and report metadata flow.
- `agent_daemon.md` explains the local-only HTTP daemon, job model, and subprocess wrapper rules.
- `ecs_runtime.md` explains the Bevy ECS schedule spine, built-in system sets, and plugin registration model.
- `headless_scenario_runner.md` explains the deterministic no-window scenario execution flow.
- `platform_shell.md` explains the bootstrap `winit` client shell, input action map, fixed-step loop, and manual smoke checklist.
- `terrain_mesh_collision.md` explains the chunked terrain mesh, deterministic triangulation, static collision bake, and debug-scene bridge.
- `terrain_scalar_fields.md` explains the deterministic hero-biome field cache, debug dumps, and overlay surface.
- `verification_stack.md` explains the repo-standard `cargo xtask verify` workflow, artifacts, and toolchain choices.
- `workspace_scaffold.md` explains the initial crate graph, merge policy, and placeholder entrypoint contract.
