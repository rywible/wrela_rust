# Redwood Tree Graphs

`wr_world_gen` now owns the graph-first procedural redwood growth pass that turns trunk placements
from ecology into inspectable branch skeletons for later mesh and foliage work.

## Inputs

- A root seed, with per-tree streams derived from `trees.redwood`.
- The matching `TerrainScalarFieldSet` and `EcologicalPlacementSet`.
- A `RedwoodForestGraphConfig` that exposes the space-colonization knobs: attraction radius,
  kill radius, segment lengths, tropism, taper, buttress boost, branch culling, and canopy
  envelope proportions.

## Solve shape

The current bootstrap implementation keeps the generator inspectable:

1. Start one graph per accepted trunk placement.
2. Grow a tall trunk spine with a slight deterministic lean.
3. Scatter canopy attractors high above the lower branch cutoff.
4. Run a simplified space-colonization loop that pulls nearby nodes toward attractors while
   blending upward and radial tropism.
5. Stop when attractors are exhausted or the configured iteration budget is reached.
6. Assign radii bottom-up so parent segments always stay at least as thick as their children,
   with an optional root buttress boost across the first few depths.

The result is graph-only data: no meshes, cards, or materials yet. That keeps PR-015 focused on
deterministic skeleton generation and gives later procedural geometry tasks a stable handoff.

## Reports

`RedwoodForestGraphSet::summary_report()` exports high-level forest statistics plus a compact list
of the first generated trees for harness and snapshot coverage.

`RedwoodForestGraphSet::debug_dump()` exports ASCII front and side silhouettes plus selected node
lists for a few representative trees. That gives us a stable, text-diffable proxy for "does this
still read like a redwood?" before full render capture exists.
