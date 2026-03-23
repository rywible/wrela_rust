# Redwood Foliage Clusters

`PR-017` adds the first deterministic canopy layer on top of the branch-and-trunk meshes from
`PR-016`.

## Ownership split

- `wr_world_gen` still owns canopy intent through branch tips and canopy envelopes.
- `wr_procgeo` now derives foliage clusters, packed material parameters, and per-LOD batch reports.
- `wr_render_scene` owns the deterministic near/far canopy debug scenes.
- `wr_render_wgpu` owns the bootstrap foliage-card shader and blending path.

## Generation model

The current canopy pass stays deliberately inspectable:

1. Select every terminal branch tip from each redwood graph.
2. Derive one cluster anchor per tip from the tip axis, canopy height, and a bounded deterministic
   lateral jitter.
3. Emit a small crossed-card bundle per cluster for `hero`, `mid`, and `far` LODs.
4. Pack procedural material controls into two `u32` words so the render path can decode them
   without authored textures.
5. Report card counts, logical vertex counts, bounds, and batch estimates per LOD.

This is not trying to be final outdoor foliage shading yet. The current contract is:

- deterministic from graph data,
- controllable from a small config surface,
- cheap enough to batch,
- and testable through snapshots plus offscreen smoke renders.
