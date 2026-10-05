# World Systems Rebuild — Implementation Status

This document tracks the current implementation state of the world-systems rebuild. The architecture and behavioral contract remain in [`world-systems-rebuild.md`](world-systems-rebuild.md); this file answers only: what is implemented now, what is intentionally incomplete, and what comes next.

Implementation branch: `world-systems-rebuild`

Code baseline recorded here: `a4b62bebeb22d7a4fe63558a0a939a083d01f54c` (`Author terrain profiles for all surface biomes`). Rust validation run `37249243969` completed successfully for that baseline.

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
| 4 — Terrain | In progress | Continuous 2D base-surface terrain and base 3D density are implemented. Surface terrain profiles are authored for all current surface biomes. True 3D terrain contributions are still pending. |
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
- `src/content/biome.rs` `surfaceTerrain`

Current terrain facts:

- `TerrainQueries::base_surface_at(x, z)` exposes the continuous authoritative 2D base surface;
- `TerrainQueries::surface_at(x, z)` currently resolves the highest voxel of that base-solid field;
- `TerrainQueries::density_at(x, y, z)` currently uses the base surface as the 3D solid/empty crossing;
- bounded terrain grid sampling reuses one authoritative biome sample grid instead of rediscovering biome ownership per voxel;
- biome influence weights blend each participating biome's terrain profile;
- terrain noise is anchored in world coordinates and deterministic generator domains;
- every current surface biome now has an authored `surfaceTerrain` profile.

Current `surfaceTerrain` shape:

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

### What is not implemented yet in Phase 4

The current 3D density is still only the base-solid conversion:

```text
base surface - world Y
```

Phase 4 is not complete until the same final terrain field can compose the required true 3D contributions, including as applicable:

- caves/carving;
- overhangs;
- floating formations;
- additive masses;
- future volume-biome terrain effects;
- effective surface/column resolution that accounts for those 3D contributions without brute-force scanning the whole world height.

Cross-chunk/world-space determinism must remain unchanged when those contributions are added.

## Next concrete work

Continue **Phase 4**, not Phase 3.

The next implementation should extend the current `TerrainField`; do not create a second terrain owner and do not return to biome-layout work unless a real Phase 4 requirement exposes a biome-layout defect.

Required direction:

1. keep `base_surface_at` as the cheap authoritative 2D terrain result;
2. introduce bounded, deterministic 3D terrain contributions into the same final density field;
3. make effective column/surface queries account for those contributions without full-height brute-force scans;
4. preserve scalar/batch equivalence and world-coordinate anchoring;
5. only after Phase 4 is complete proceed to surface/material/generated-fluid composition.

## Important non-regression rules

- No legacy worldgen implementation may be restored for convenience.
- No semantic generation region or chunk-owned biome/terrain truth.
- No separate land/ocean ownership model.
- No hydrology subsystem; rivers remain generic connected Structures/connectors.
- No duplicated biome map, terrain sampler, or query resolver.
- No compatibility shims for deleted generation contracts unless explicitly requested.
- No `cargo test` in agent workflows/CI unless explicitly requested for the current task.
