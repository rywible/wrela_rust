# Seed Graph

`PR-008` establishes the deterministic seed topology that later procedural crates are expected to consume.

## Standard topology

Every scenario starts from one explicit root seed. The standard v0 graph derives the following stable paths:

- `terrain`
- `ecology`
- `trees`
- `wraiths`
- `combat`
- `combat.scenarios`
- `vfx`

All derived paths use stable byte-order hashing from the parent seed plus the local label. No gameplay-facing deterministic path may depend on hash map iteration order or global RNG state.

## Named seed config packs

`wr_core::SeedConfigPack` is the serializable override surface for deterministic seed work. Packs are intentionally small and diff-friendly:

- they have a schema version,
- they have a required `pack_name`,
- they may override one or more known seed paths with explicit hex seeds.

Overrides are path-scoped. Changing `terrain` only changes the `terrain` branch. Changing `combat` also changes `combat.scenarios`, because that path derives from `combat`.

## Report contract

`wr_telemetry::SeedInfo` now carries:

- the root seed label and value,
- the optional scenario stream,
- the derived seed path list,
- the applied config pack name and explicit overrides.

Headless scenario reports emit this structure so later world-gen, combat, and VFX tasks can explain exactly which deterministic inputs produced a run.
