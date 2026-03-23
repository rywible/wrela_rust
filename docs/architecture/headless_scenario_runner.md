# Headless Scenario Runner

`PR-003` introduces the first deterministic, no-window execution path for the repo: `wr_headless` and the matching `cargo xtask run-scenario` wrapper.

## Purpose

The headless runner exists so agents can validate gameplay-facing contracts without depending on a visible client, GPU state, or log scraping.

The current bootstrap-plus-runtime form intentionally does three things:

1. load a `.ron` scenario contract,
2. simulate a fixed number of steps in a deterministic world stub,
3. emit a machine-readable terminal report even when the scenario fails.

The harness surface remains narrow, but it now runs on top of the repo's explicit ECS schedule spine from `PR-007`.

## Scenario file contract

Scenario files are authored in RON and deserialize into `wr_tools_harness::ScenarioRequest`.

The current contract includes:

- schema version,
- canonical scenario path,
- simulation rate,
- fixed step count,
- root seed metadata,
- spawned actors,
- scripted inputs,
- assertions, optionally pinned to a simulation frame.

Assertions with a `frame` are evaluated immediately after that frame's fixed step. Assertions without a `frame` are evaluated once the run completes. The runner stops at the first failing assertion and still writes `terminal_report.json`.

That means `frame: 0` sees the world after the first fixed step has completed, not the pre-simulation state.

## Execution flow

The current no-window path is:

`RON scenario -> wr_headless loader -> wr_game headless summary -> reports/harness/run-scenario/<run_id>/terminal_report.json`

`wr_game` owns the fixed-step loop and assertion evaluation.

`wr_ecs` now provides the runtime schedule spine used by the headless path. The current headless world still tracks only the minimum gameplay state needed for smoke scenarios:

- actor spawns stored as ECS components,
- active scripted inputs stored in ECS resources,
- simulated frame count tracked through the fixed schedules,
- event records used for deterministic hashing.

The fixed-step order is now:

`FixedPrePhysics -> FixedPhysics -> FixedGameplay -> FixedPostGameplay`

The extraction path reserved for later render work is also explicit:

`Extract -> RenderPrep`

`wr_world_seed` provides stable root-seed parsing plus deterministic sub-stream derivation for actor signatures and report hashing.

## Report shape

`terminal_report.json` for `run-scenario` now contains a `ScenarioExecutionReport`.

That report carries:

- repo/runtime metadata,
- seed and scenario identity,
- pass/fail result envelope,
- fixed-step execution metrics,
- per-assertion outcomes,
- a deterministic hash derived from simulation state rather than timestamps,
- stable artifact paths.

The deterministic hash is the main bootstrap proof that repeated runs of the same scenario and seed stayed identical even though timestamps and run IDs differ across executions.

Run metadata timestamps are intentionally ambient observability fields and are excluded from the determinism hash. For fully reproducible artifact paths, callers should pass an explicit `--run-id` instead of relying on the timestamp-derived default.
