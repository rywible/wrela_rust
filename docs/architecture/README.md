# Architecture Notes

This directory holds architecture documents that explain how the current repo shape maps to the planned runtime.

- `harness_contract.md` explains the machine-readable harness request/report contract.
- `agent_daemon.md` explains the local-only HTTP daemon, job model, and subprocess wrapper rules.
- `headless_scenario_runner.md` explains the deterministic no-window scenario execution flow.
- `platform_shell.md` explains the bootstrap `winit` client shell, input action map, fixed-step loop, and manual smoke checklist.
- `verification_stack.md` explains the repo-standard `cargo xtask verify` workflow, artifacts, and toolchain choices.
- `workspace_scaffold.md` explains the initial crate graph, merge policy, and placeholder entrypoint contract.
