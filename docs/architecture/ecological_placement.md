# Ecological Placement

`wr_world_gen` now owns the deterministic ecological placement solve that turns terrain scalar
fields into inspectable trunk, understory, and deadfall anchor layouts.

## Inputs

- A root seed, with per-candidate jitter and ordering derived from stable stream labels.
- A `TerrainScalarFieldSet` generated from the matching biome dimensions.
- An `EcologicalPlacementConfig` that defines densities, spacing, hero-corridor protection, slope
  limits, and debug-map resolution.

## Solve shape

The solve stays intentionally inspectable:

1. Build one jittered candidate per spacing-sized grid cell for each placement kind.
2. Sample terrain fields at each candidate point.
3. Score candidates from ecological heuristics:
   - trunks prefer canopy opportunity, moderate moisture, low slope, and clear hero paths,
   - understory prefers moisture, fog, mid-canopy opportunity, and manageable slope,
   - deadfall anchors prefer deadfall probability, drainage, moderate slope, and clear hero paths.
4. Sort candidates by suitability plus a deterministic tie-breaker.
5. Greedily accept candidates while enforcing same-kind spacing, trunk competition clearances, hero
   corridor exclusions for major placements, and per-kind slope ceilings.

The result is deterministic per seed and configuration while still reading as blue-noise-style
ecological placement instead of paint-by-hand authoring.

## Reports

`EcologicalPlacementSet::summary_report()` exports per-kind targets, accepted counts, rejection
reasons, and density/spacing statistics for harness inspection.

`EcologicalPlacementSet::debug_dump()` exports occupancy maps for each placement kind plus a
forbidden hero-corridor map. Those maps are quantized to a stable resolution so later harness and
look-dev tools can diff them without scraping ad-hoc logs.
