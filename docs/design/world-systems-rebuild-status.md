# World Systems Rebuild — Implementation Status

This document tracks the current implementation state of the world-systems rebuild. The architecture and behavioral contract remain in [`world-systems-rebuild.md`](world-systems-rebuild.md); this file answers only: what is implemented now, what is intentionally incomplete, and what comes next.

Implementation branch: `world-systems-rebuild`

Code baseline recorded here: `c70b205a259359f0fffd8deb7630a3d12fe42337` (`Add terrain debug CLI`). Rust validation run `37252948392` completed successfully for that baseline.

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
| 4 — Terrain | In progress | Continuous 2D base-surface terrain, final 3D density composition, bounded subtractive caves, biome-authored bounded floating masses, effective surface resolution, dense XYZ density sampling, and an authoritative-query terrain debug renderer are implemented. Visual inspection and any remaining required true-3D forms are still pending. |
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
- `src/world/generator/terrain_debug.rs`
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
- every current surface biome has an authored `surfaceTerrain` profile;
- `src/world/generator/terrain_debug.rs` consumes only `TerrainQueries` and does not reimplement terrain semantics;
- `--terrain-debug` renders an X/Z effective-surface image, an X/Y final-density slice at the selected Z, and JSON metadata from the same authoritative queries;
- the density slice distinguishes base solid, cave carve, additive mass, base-surface crossing, and effective additive crossing, while the surface map highlights columns whose effective top rises above the base surface.

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

The final density field now composes both subtractive and additive bounded 3D terrain, and the authoritative-query debug renderer exists. Phase 4 is still not complete because the debug output has not yet been used to complete visual validation of the current terrain field. Remaining work includes, as applicable:

- inspect floating formation shape and biome-edge retreat;
- inspect cave crossings and accidental surface breakthroughs;
- inspect cross-chunk/cross-boundary seams using matching world-space windows;
- add overhangs or other non-floating additive forms only where the authored terrain contract actually requires them;
- future volume-biome terrain effects remain deferred until volume-biome authoring exists.

Cross-chunk/world-space determinism must remain unchanged as those contributions are added.

## Next concrete work

Continue **Phase 4**, not Phase 3.

The next work should use the terrain debug path rather than inventing another terrain sampler or resolver.

Required direction:

1. render representative `--terrain-debug` windows for base terrain, floating-island boundaries, and cave-heavy slices;
2. inspect the generated surface/density images and JSON summaries for crossings and seams;
3. correct defects in the existing final `TerrainField` if the validation exposes any;
4. add another true-3D terrain form only if the visual/behavioral contract demonstrates it is actually required;
5. preserve scalar/batch equivalence and world-coordinate anchoring;
6. once terrain validation is satisfied, close Phase 4 and proceed to Phase 5 surface/material/generated-fluid composition.

## Important non-regression rules

- No legacy worldgen implementation may be restored for convenience.
- No semantic generation region or chunk-owned biome/terrain truth.
- No separate land/ocean ownership model.
- No hydrology subsystem; rivers remain generic connected Structures/connectors.
- No duplicated biome map, terrain sampler, or query resolver.
- No compatibility shims for deleted generation contracts unless explicitly requested.
- No `cargo test` in agent workflows/CI unless explicitly requested for the current task.
