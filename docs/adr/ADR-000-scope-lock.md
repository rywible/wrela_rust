# ADR-000: Wrela v0 Scope Lock

- Status: accepted
- Date: 2026-03-22
- Decision owners: repo maintainers and autonomous agents working under `AGENTS.md`

## Context

Wrela v0 is intentionally narrow. The project needs a clear scope lock before autonomous implementation begins so that agents do not widen the runtime into a general-purpose engine.

## Decision

For v0, the shipped slice is locked to:

- one non-streaming hero biome cell,
- one fixed hero time-of-day,
- one enemy archetype,
- one player weapon,
- one 1v1 duel loop,
- no imported world meshes,
- no imported skeletal characters,
- no imported animation clips,
- no open-world streaming,
- no day/night cycle.

## Consequences

- Future roadmap tasks must serve this slice directly.
- Generality is welcome only when it clearly improves determinism, simplicity, observability, or frame stability for the shipped slice.
- Scope exceptions require a later ADR and explicit human approval.
