# Workspace Scaffold

## Purpose

This scaffold creates the full package topology early so later tasks can stay inside one subsystem crate plus tests and docs.

## Merge policy

- Subsystem work should land in its owning crate whenever possible.
- `wr_game` is integration-only and should compose subsystems instead of absorbing subsystem logic.
- `apps/*` are thin shells around the composed runtime and tooling surfaces.
- `xtask` is the repo automation entrypoint and should remain the stable command front door.

## Placeholder entrypoint contract

Each package exposes a placeholder `init_entrypoint()` function during scaffold phase.

- For subsystem crates, this is the temporary plugin/init contract.
- For `wr_game`, it marks the composition boundary.
- For app shells, it marks the shell boundary while keeping logic out of `main`.
- For `xtask`, it marks the tooling entrypoint while command surface remains intentionally small.

A later task may evolve these placeholders into richer plugin types, but new packages should continue to expose explicit init/plugin entrypoints rather than self-registering implicitly.

## Feature-flag conventions

- Every package starts with `default = []`.
- Feature flags must be additive and narrow.
- Do not introduce a workspace-wide "kitchen sink" feature.
- Backend or heavy optional features should wait for the task that actually needs them.

## Directory conventions

- `docs/adr/` stores accepted and proposed ADRs.
- `docs/architecture/` stores current boundary notes.
- `docs/tuning/` stores tuning workflow notes and pack conventions.
- `scenarios/`, `reports/`, `generated-cache/`, `baselines/`, and `tweak_packs/` exist early so later tasks can write stable artifacts without inventing paths.
