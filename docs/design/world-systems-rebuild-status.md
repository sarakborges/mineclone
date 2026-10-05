# World Systems Rebuild — Implementation Status

This document tracks the current implementation state of the world-systems rebuild. The architecture and behavioral contract remain in [`world-systems-rebuild.md`](world-systems-rebuild.md); this file answers only: what is implemented now, what is intentionally incomplete, and what comes next.

Implementation branch: `world-systems-rebuild`

Code baseline recorded here: `7fab2753ed651416c2212358cbb146357349acaa` (`Add bounded floating terrain contributions`). Rust validation run `37252455803` completed successfully for that baseline.

## Validation policy

Repository validation follows root `AGENTS.md`.

- Do not run or add `cargo test` unless the user explicitly requests that command for the current task.
- Default Rust validation is Clippy with `-D warnings`, `cargo check --locked`, and applicable content/localization/GLB audits.
- The rebuild plan may describe behavioral invariants and regression coverage, but that does not override the repository command policy.

## Phase status

| Phase | Status | Current state |
| --- | --- | --- |
| 0 — Impact audit | Complete | External consumers and required capability boundaries are documented in the rebuild plan. |
| 1 — Cleanup | Complete | Legacy biome/world-generation/loading ownership and transitional biome presentation bridges were removed. Preserved runtime systems remain separate from the new generator. |
| 2 — Generation foundation | Complete | New generation foundation owns world-space coordinates, deterministic entropy/domains, immutable dimension snapshot state, direct far-coordinate queries, and scalar/batch primitives. |
| 3 — Biome Layout | Complete for current authored content | New surface layout, biome queries, influences, search, `regionSize`, `cannotBorder`, and authoritative biome-map rendering exist. No volume-biome content is currently authored, so the volume query returns no override and effective biome falls back to surface ownership. |
| 4 — Terrain | In progress | Continuous 2D base-surface terrain, final 3D density composition, bounded subtractive caves, biome-authored bounded floating masses, effective surface resolution, and dense XYZ density sampling are implemented. Terrain visual/debug validation and remaining true-3D forms are still pending. |
| 5 — Surface/materials/generated fluids | Pending | Not started on the new stack. |
| 6 — Structures/features | Pending | Generic Structure primitives are preserved, but the new generation-side integration is not implemented yet. |
| 7 — Chunk synthesis | Pending | No authoritative new-generator-to-`VoxelChunk` materialization path yet. |
| 8 — Consumer integration | Pending | `/locate`, warp, spawn, portals and streaming still need reconnection to the new generator capabilities. |
| 9 — Persistence | Pending | New all-materialized-chunks persistence contract is documented but not implemented. |
| 10 — Loading pipeline | Pending | Old loading ownership was removed; replacement pipeline has not been implemented. |
| 11 — Loading screen | Pending | Replacement UI has not been implemented. |
| 12 — End-to-end/performance | Pending | Begins after the new stack has an end-to-end playable path. |

## Phase 1 — completed cleanup

The rebuild no longer keeps the previous world generator alive behind adapters.

Removed or retired during cleanup included the old biome field/generation owners, old generation regions/schedulers, old setup/loading orchestration, old generated-fluid frontier coupling, old initial-mesh presentation scheduling, old worldgen persistence metadata, and temporary biome/tint bridges that existed only to keep legacy generation behavior visible.

Infrastructure intentionally preserved includes runtime voxel/chunk state, residency foundations, meshing/remeshing, lighting, dynamic fluids, rendering, generic Structure authoring primitives, gameplay systems, entities, inventory, crafting, and storage.

## Phase 2 — completed generation foundation

The new generator foundation lives under `src/world/generator/` and is independent from the deleted worldgen implementation.

Current foundation properties:

- immutable generation snapshot per active dimension;
- deterministic named entropy domains owned by the new generator;
- generation-specific world coordinate/request types;
- direct coordinate access with no source-to-destination traversal;
- scalar and bounded grid/volume sampling primitives;
- deterministic results independent of query history or generation order;
- runtime installation/removal of the immutable `WorldGenerator` snapshot at world/dimension lifecycle boundaries;
- no requirement to materialize a `VoxelChunk` merely to answer generated-world queries.

## Phase 3 — completed surface Biome Layout

Authoritative implementation:

- `src/world/generator/biome.rs`
- `src/world/generator/biome_map.rs`
- `src/content/biome.rs`

The new biome authoring contract uses `surfaceLayout` and owns only layout concerns:

```json
{
  "surfaceLayout": {
    "weight": 1.0,
    "regionSize": {
      "min": 384,
      "max": 768
    },
    "cannotBorder": []
  }
}
```

Implemented surface-query capabilities include:

- one authoritative primary biome per X/Z point;
- normalized variable influence collection for continuous transitions;
- scalar surface sampling;
- bounded area/grid sampling;
- surface biome search;
- region-size metadata;
- deterministic formation geometry and compatibility filtering;
- symmetric `cannotBorder` evaluation;
- effective XYZ biome resolution that currently falls back to the surface field because no volume-biome field/content is authored yet.

### Authoritative biome map viewer

`src/world/generator/biome_map.rs` is the single biome-map implementation. A duplicate map path was removed rather than maintained in parallel.

The viewer consumes `BiomeQueries`, the same source used by generation. It supports:

- deterministic PNG output;
- stable biome colors;
- center, scale and output resolution;
- boundary overlay;
- influence/blend rendering;
- JSON legend containing biome IDs and `regionSize` metadata.

The viewer must remain a consumer of the authoritative layout. It must never become a second biome resolver.

## Phase 4 — terrain currently in progress

Authoritative implementation currently being built:

- `src/world/generator/terrain.rs`
- `src/content/biome.rs` `surfaceTerrain` and `terrain3d`

Current terrain facts:

- `TerrainQueries::base_surface_at(x, z)` exposes the continuous authoritative 2D base surface and remains unchanged by true-3D contributions;
- `TerrainQueries::density_at(x, y, z)` composes base solidity, the bounded subtractive cave field, and biome-authored additive floating density inside the same final `TerrainField`;
- cave noise is deterministic, world-coordinate anchored, and limited to a finite depth band below the base surface;
- `terrain3d.floatingFormation` authors explicit absolute vertical bounds, horizontal/detail scales, coverage, roughness, and density scale for bounded additive floating masses;
- the current Overworld `floating_islands` biome authors a floating formation in world Y `200..280`;
- floating formation shape and biome-boundary retreat use deterministic world-coordinate noise and authoritative biome influence weights;
- `TerrainQueries::surface_at(x, z)` resolves the highest effective solid crossing by scanning only the finite vertical candidate bounds exposed by active additive contributions, never the full world height;
- bounded surface-grid sampling uses the same effective surface resolver as scalar queries;
- `TerrainQueries::sample_density_volume(...)` provides dense XYZ batch sampling while reusing one authoritative biome sample and base-surface result per X/Z column;
- biome influence weights blend each participating biome's base terrain profile;
- every current surface biome has an authored `surfaceTerrain` profile.

Current base-surface authoring shape:

```json
{
  "surfaceTerrain": {
    "baseHeightOffset": 8.0,
    "macroAmplitude": 18.0,
    "macroScale": 640,
    "detailAmplitude": 4.0,
    "detailScale": 96
  }
}
```

Current bounded floating authoring shape:

```json
{
  "terrain3d": {
    "floatingFormation": {
      "minY": 200,
      "maxY": 280,
      "horizontalScale": 112,
      "detailScale": 40,
      "coverage": 0.55,
      "roughness": 0.18,
      "densityScale": 28.0
    }
  }
}
```

### What is not implemented yet in Phase 4

The final density field now composes both subtractive and additive bounded 3D terrain, but Phase 4 is not complete. Remaining work includes, as applicable:

- overhangs and other non-floating additive forms where authored terrain requires them;
- future volume-biome terrain effects;
- terrain visual/debug validation of final density, crossings and seams before materials/features are layered on top.

Cross-chunk/world-space determinism must remain unchanged as those contributions are added.

## Next concrete work

Continue **Phase 4**, not Phase 3.

The next implementation should extend the current `TerrainField`; do not create a second terrain owner and do not return to biome-layout work unless a real Phase 4 requirement exposes a biome-layout defect.

Required direction:

1. keep `base_surface_at` as the cheap authoritative 2D terrain result;
2. add a terrain visual/debug path that consumes the same `TerrainQueries` and can inspect final density/effective surfaces without becoming a second resolver;
3. use that validation to inspect floating formations, cave crossings and cross-boundary/seam behavior;
4. add any remaining true-3D terrain forms required to satisfy Phase 4 from the same final density owner;
5. preserve scalar/batch equivalence and world-coordinate anchoring;
6. only after Phase 4 is complete proceed to surface/material/generated-fluid composition.

## Important non-regression rules

- No legacy worldgen implementation may be restored for convenience.
- No semantic generation region or chunk-owned biome/terrain truth.
- No separate land/ocean ownership model.
- No hydrology subsystem; rivers remain generic connected Structures/connectors.
- No duplicated biome map, terrain sampler, or query resolver.
- No compatibility shims for deleted generation contracts unless explicitly requested.
- No `cargo test` in agent workflows/CI unless explicitly requested for the current task.
