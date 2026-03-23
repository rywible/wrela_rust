# ECS Runtime

`PR-007` introduces the first real gameplay schedule spine for the repo: `wr_ecs` now wraps standalone `bevy_ecs` with explicit schedule phases and a plugin registration pattern that keeps subsystem wiring out of global state.

## Purpose

The ECS runtime exists to give every later subsystem a stable place to register resources and systems without reaching across crate boundaries or teaching integration crates hidden magic.

The current runtime contract is intentionally narrow:

1. `wr_ecs::EcsRuntime` owns a `bevy_ecs::World` plus pre-created schedules.
2. Integration code registers plugins explicitly.
3. Fixed-step and render extraction work run in stable schedule order.

## Schedule contract

The runtime creates these schedules up front:

- `Startup`
- `FixedPrePhysics`
- `FixedPhysics`
- `FixedGameplay`
- `FixedPostGameplay`
- `Extract`
- `RenderPrep`
- `Shutdown`

`run_fixed_frame(frame)` runs the four fixed schedules in order after updating the current frame resource.

`run_render_frame()` runs `Extract` and `RenderPrep` in order.

In debug builds, schedule ambiguity detection is enabled so conflicting mutable access patterns are surfaced early while the runtime is still small.

## Built-in system sets

Every schedule is configured with the same core system-set vocabulary:

- `WorldGen`
- `Combat`
- `Ai`
- `RenderExtract`
- `Tooling`

That keeps later crate plugins speaking the same scheduling language even before the content systems exist.

## Plugin model

Subsystems register through the `GamePlugin` trait:

- plugins name themselves explicitly,
- duplicate plugin names are rejected,
- plugins receive `&mut EcsRuntime` and register their own resources and systems,
- integration crates decide which plugins are present and in what order they are added.

`wr_game::compose_game_runtime` is the composition entrypoint for building a runtime from an explicit plugin list.

## Headless runner relationship

The headless scenario path still uses a narrow gameplay slice, but it is no longer a hand-rolled loop. `HeadlessScenarioWorld` now stores its state in ECS resources/components and advances through the fixed schedules, which keeps the harness behavior compatible while making later runtime work additive instead of a rewrite.
