# Terrain Scalar Fields

`PR-012` establishes the deterministic ecological field layer that later terrain, placement, and look-dev tasks build on.

## Purpose

The field set is the stable low-frequency substrate for the hero biome. Later generators are expected to sample it instead of re-deriving ad hoc terrain heuristics.

The current field cache includes:

- `height`
- `slope`
- `drainage`
- `moisture`
- `fog`
- `canopy_opportunity`
- `deadfall_probability`
- `hero_path_bias`

## Generation model

`wr_world_gen::TerrainScalarFieldSet::generate` consumes an explicit terrain seed plus a diff-friendly config and bakes a deterministic sample cache for the full `512m x 512m` hero cell.

The cache is CPU-authored from a small stack of deterministic `wr_math::FractalNoise2` sources:

- landform and ridge noise shape the broad height field,
- finite-difference probes derive slope from the height surface,
- moisture, canopy, and deadfall blend the terrain shape with independent sub-stream noise,
- hero-path bias reserves a readable traversal corridor through the biome.

Sampling later stays cheap and stable through bilinear interpolation over the baked cache.

## Debug surfaces

The field API exposes three machine-verifiable surfaces:

- `summary_report()` for per-field min/max/mean/stddev summaries,
- `debug_dump(resolution)` for compact quantized scalar grids,
- `render_overlay(field, resolution)` for RGBA overlay buffers that a later renderer or tool UI can draw directly.

The world-gen crate intentionally stops at deterministic data products here. It does not yet decide how those overlays are composited into a client frame.
