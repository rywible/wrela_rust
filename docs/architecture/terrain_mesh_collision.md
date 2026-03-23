# Terrain Mesh And Collision

`PR-013` turns the scalar-field substrate from `PR-012` into deterministic geometry, collision, and a renderable terrain debug scene.

## Ownership split

- `wr_world_gen` still owns the scalar field cache and now exposes stable grid-point helpers for downstream deterministic consumers.
- `wr_procgeo` owns chunked terrain mesh generation, per-vertex normals/tangents, crack-free seam handling, and mesh statistics.
- `wr_physics` owns the static terrain collider built from the mesh plus sample ray and height queries.
- `wr_render_scene` owns the debug-scene bridge that projects terrain surface, normals, tangents, and collision wireframe into the existing extracted-scene/debug-triangle contract.

## Deterministic mesh model

`wr_procgeo::TerrainMeshAtlas::build` samples the full cached terrain grid and then partitions the shared vertex lattice into fixed-size chunks.

Important constraints:

- chunk borders reuse the same global grid vertices, so adjacent chunks share identical edge positions, normals, and tangents,
- triangulation uses a checkerboard diagonal keyed by global cell coordinates, so seam decisions are deterministic across chunk boundaries,
- mesh and collider stats are emitted as stable snapshot-friendly reports rather than inferred from logs.

## Collision model

`wr_physics::TerrainCollider` currently bakes a static triangle soup from the terrain mesh and exposes deterministic downward sampling through explicit ray tests.

That keeps the current proof path honest:

- render geometry and collision come from the same baked mesh surface,
- headless verification can raycast or sample terrain height without any live client dependencies,
- later kinematic/player tasks can replace the broad query strategy without changing the report surface introduced here.

## Debug rendering bridge

The bootstrap client still uses the debug-triangle render path, so the terrain scene is intentionally a projected debug view rather than the final outdoor renderer.

`wr_render_scene::canonical_hero_terrain_debug_scene` exists to prove three things before later render work lands:

- the canonical seed can be turned into renderable terrain geometry,
- normals, tangents, and collision wireframe have a stable extraction path,
- the live client and offscreen renderer can both consume the same extracted scene.
