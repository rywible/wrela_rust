# Redwood Tree Meshes

`PR-016` turns the inspectable redwood skeletons from `wr_world_gen` into deterministic trunk and
major-branch geometry that downstream rendering can batch without inventing a second tree shape.

## Ownership split

- `wr_world_gen` continues to own growth, canopy placement intent, and per-node radii.
- `wr_procgeo` now owns the graph-to-mesh conversion, bark displacement, LOD generation, and
  mesh reports.
- `wr_render_scene` owns the bootstrap forest debug scene that proves a tree batch can be rendered
  together with the terrain substrate.

## Mesh model

The current mesh pass deliberately stays inspectable and deterministic:

1. Filter the graph down to the trunk and major branches using a radius floor.
2. Build one shared local frame per included node from its parent/child directions.
3. Wrap each node in a ring of generalized-cylinder vertices.
4. Apply bark ridges as a deterministic radial perturbation driven by angle, path distance, tree
   index, and depth.
5. Stitch parent/child rings into tube segments and cap the root plus every surviving tip.
6. Rebuild the same tree three times with different radial segment counts for `hero`, `mid`, and
   `far` LODs.

This is intentionally not a full manifold branch-joint solver yet. The current contract is
"closed, finite, stable, and batchable" so later foliage, lighting, and wraith tasks have a
reliable forest scaffold to build on.

## Reports and debug proof

`RedwoodForestMeshSet::report()` emits aggregate and per-tree counts for included nodes, capped
tips, triangle totals, mean/max radii, and silhouette extents.

`wr_render_scene::canonical_redwood_forest_debug_scene()` renders a deterministic forest patch
through the existing debug-triangle path so the repo can prove the new meshes survive extraction
and offscreen rendering before the outdoor renderer grows more specialized material support.
