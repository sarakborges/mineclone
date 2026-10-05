# World Systems Rebuild — Implementation Status

This document tracks the current implementation state of the world-systems rebuild. The architecture and behavioral contract remain in [`world-systems-rebuild.md`](world-systems-rebuild.md); this file records only what is implemented now, what remains, and the next concrete cut.

Implementation branch: `world-systems-rebuild`

Code baseline recorded here: `4ed132a92516c1213f540b7649ff09062bddaed5` (`Keep generated structure validation scoped`), including `7b20dee4d1c331b62d3e550ead42df3e2531e2df` (`Resolve root structure placement conflicts`). Rust validation run `37299795207` completed successfully for that baseline. Terrain visual/invariant validation remains available through the on-demand terrain debug path.

## Validation policy

Repository validation follows root `AGENTS.md`.

- Do not run or add `cargo test` unless the user explicitly requests that command for the current task.
- Default Rust validation is Clippy with `-D warnings`, `cargo check --locked`, and applicable content/localization/GLB audits.
- Visual/debug validation is an on-demand consumer capability, not a permanent CI fixture-rendering loop.
- A milestone is not complete while the exact delivered SHA has pending, failed, or unobservable CI.

## Phase status

| Phase | Status | Current state |
| --- | --- | --- |
| 0 — Impact audit | Complete | External consumers and required capability boundaries are documented. |
| 1 — Cleanup | Complete | Legacy biome/world-generation/loading ownership and transitional generation bridges were removed; preserved runtime systems remain separate. |
| 2 — Generation foundation | Complete | Immutable generation snapshots, deterministic named entropy, world-coordinate queries, and scalar/bounded sampling primitives are implemented. |
| 3 — Biome Layout | Complete for current authored content | Authoritative surface layout, influences, search, `regionSize`, `cannotBorder`, and biome-map rendering are implemented. No volume-biome content is currently authored. |
| 4 — Terrain | Complete for current authored content | Authoritative continuous base surface, final 3D density, caves, floating masses, bounded effective surfaces, scalar/batch queries, and terrain debug validation are implemented. |
| 5 — Surface/materials/generated fluids | Complete for current authored content | Solid layers, deterministic material patches, Ocean water, swamp puddles, volcano lava pools, scalar/batch fluid queries, and the generated-fluid runtime-frontier boundary are implemented. |
| 6 — Structures/features | In progress | `StructureField` owns deterministic root placement/query plus priority, conflict-group, reservation, and overlap arbitration for the first authored Plains Structures. StructureSets, connectors, remaining biome placements, and final placement-graph seam/order validation are still pending. |
| 7 — Chunk synthesis | Pending | No authoritative new-generator-to-`VoxelChunk` materialization path yet. |
| 8 — Consumer integration | Pending | `/locate`, warp, spawn, portals, and streaming still need reconnection to the new generator capabilities. |
| 9 — Persistence | Pending | New all-materialized-chunks persistence contract is documented but not implemented. |
| 10 — Loading pipeline | Pending | Old loading ownership was removed; replacement pipeline has not been implemented. |
| 11 — Loading screen | Pending | Replacement UI has not been implemented. |
| 12 — End-to-end/performance | Pending | Begins after the new stack has an end-to-end playable path. |

## Completed generation ownership

### Phase 2 — Generation foundation

The generation foundation lives under `src/world/generator/` and is independent from runtime chunk state.

Current invariants:

- one immutable seed/dimension snapshot per active generator;
- deterministic named entropy domains;
- direct world-coordinate access with no generation-order dependency;
- scalar and bounded area/volume sampling;
- no semantic generation region or chunk-owned truth;
- no need to materialize a `VoxelChunk` to answer generated-world queries.

### Phase 3 — Biome Layout

Authoritative implementation:

- `src/world/generator/biome.rs`
- `src/world/generator/biome_map.rs`
- `src/content/biome.rs`

The surface biome field owns primary identity, influences, region sizing, adjacency constraints, bounded sampling, and biome search. The map viewer consumes `BiomeQueries`; it is not another layout resolver. Effective XYZ biome lookup currently falls back to surface ownership because no volume-biome content is authored.

### Phase 4 — Terrain

Authoritative implementation:

- `src/world/generator/terrain.rs`
- `src/world/generator/terrain_debug.rs`
- `src/content/biome.rs` `surfaceTerrain` and `terrain3d`

Current terrain facts:

- `base_surface_at(x, z)` is the cheap authoritative 2D base surface;
- `density_at(x, y, z)` is the one final terrain-density answer;
- caves are bounded subtractive contributions;
- biome-authored floating formations are bounded additive contributions in the same `TerrainField`;
- `surface_at(x, z)` resolves the effective top using finite candidate bounds rather than scanning world height;
- scalar, grid, and dense volume paths share the same semantics and world-coordinate anchoring;
- the terrain debug path validates scalar/batch equivalence, overlapping request seams, repeated-query determinism, and effective-surface crossings.

No currently authored terrain requires another true-3D form. Future volume-biome or overhang forms must compose into this same owner.

### Phase 5 — Surface/material/generated-fluid composition

Authoritative implementation:

- `src/world/generator/material.rs`
- `src/world/generator/generated_fluid.rs`
- `src/content/biome.rs` `surfaceLayers` and `surfaceLayers.patch`
- `src/content/dimension/types.rs` `generatedOcean` and `generatedSurfaceFluids`
- `data/dimensions/overworld/dimension.json`
- `src/world/fluid_updates.rs` for the explicit generated-frontier runtime handoff

Current contract:

- `TerrainField` alone owns solid/empty geometry;
- `MaterialField` classifies already-solid terrain into generated block identities;
- `GeneratedFluidField` alone owns initial generated-fluid spatial rules;
- solid and fluid scalar/batch queries consume the same biome/terrain facts;
- deep cave walls/floors remain core material rather than becoming topsoil merely because they are exposed;
- swamp surface patches deterministically interleave grass/dirt/mud;
- Ocean water fills only empty voxels above the authored Ocean floor through `seaLevel`, so cave voids below the floor stay dry;
- swamp puddles are shallow Terrain cuts filled by generated water, so water genuinely replaces/interleaves with the surface instead of occupying a still-solid voxel;
- Volcano currently authors sparse two-voxel-deep deterministic surface lava pools through the same generated-fluid owner;
- Volcano does not currently author a crater/caldera Terrain form, so these pools are not mislabeled as crater lava;
- generated fluid is initial world formation and filled voxels are never bulk-scheduled into the runtime solver;
- `enqueue_generated_fluid_frontier(...)` is the explicit future Phase 7 handoff for exposed runtime continuation targets only.

The currently authored generated-fluid shape includes:

```json
{
  "generatedOcean": {
    "biome": "asteria:overworld/ocean",
    "fluid": "asteria:water"
  },
  "generatedSurfaceFluids": [
    {
      "biome": "asteria:overworld/swamp",
      "fluid": "asteria:water",
      "spacing": 18,
      "radius": 5,
      "jitter": 3,
      "chance": 0.75,
      "depth": 1
    },
    {
      "biome": "asteria:overworld/volcano",
      "fluid": "asteria:lava",
      "spacing": 96,
      "radius": 10,
      "jitter": 12,
      "chance": 0.4,
      "depth": 2
    }
  ]
}
```

## Phase 6 — Structures/features in progress

Authoritative generation-side implementation now begins at:

- `src/world/generator/structure.rs`
- preserved generic authoring/runtime definitions in `src/content/structure.rs`
- preserved restriction definitions in `src/content/structure_rules.rs`
- preserved composition definitions in `src/content/structure_set.rs`
- `src/content/dimension/types.rs` `generatedSurfaceStructures`
- `data/dimensions/overworld/dimension.json`

### Implemented Structure foundation

`StructureField` is the single new generation-side owner for deterministic Structure placement. Runtime `/place structure` remains a manual mutation path and is not the generation owner.

Implemented now:

- root placements use deterministic seed/domain-addressed world-space lattice cells with authored `spacing`, `chance`, and `jitter`;
- rules reference either a concrete Structure or an existing Structure group; group variants are frozen into the immutable generator snapshot and selected deterministically;
- authored rotation support is selected deterministically from the Structure definition;
- root candidates require the authoritative primary surface biome instead of resolving biome boundaries independently;
- ground fitting consumes `TerrainQueries::surface_at` and the existing Structure support/full-footprint rules;
- authored min/max slope, min/max Y, `groundBlocks`, `requiredBiomeCoverage`, dry-ground, fluid-forbid, and block/fluid proximity constraints consume authoritative biome/terrain/material/generated-fluid queries;
- root arbitration is intent-based: a candidate is rejected whenever a directly overlapping higher-ranked conflicting intent exists, regardless of whether that higher-ranked intent would itself later survive another conflict;
- root ranking is deterministic: authored `priority` descending, then placement reference, biome id, root anchor X/Z, and placement Y ascending;
- `reserveSpace` is directional: only a higher-ranked reserving root can reject a lower-ranked overlap solely by reservation;
- shared `conflictGroups` are symmetric once the deterministic ranking selects the higher intent;
- arbitration requires actual 3D placement overlap, using full horizontal and vertical Structure bounds rather than request/chunk bounds;
- competitor discovery is performed from complete logical placement bounds, so a higher-ranked placement outside the requesting rectangle can still reject a lower-ranked placement crossing that boundary;
- `StructureQueries::placements_intersecting(...)` enumerates only relevant world-space candidate cells, resolves complete logical placements, arbitrates them, then reports surviving placements whose actual bounds intersect the requested rectangle;
- querying a neighboring/requesting area therefore does not clip a logical Structure to that area or change which root wins an overlap;
- `StructureQueries::find_nearest(...)` searches the same authoritative root placement owner and filters through the same arbitration instead of creating a locate-specific resolver;
- `generatedSurfaceStructures` references are validated at content-load time: referenced biome must exist as a surface biome in the same dimension and the Structure/Structure-group reference must resolve;
- runtime world installation freezes the loaded `StructureRegistry` into the generator-side owner; biome/terrain diagnostic tools that do not consume Structures remain independent of Structure content.

Current root authoring shape:

```json
{
  "generatedSurfaceStructures": [
    {
      "biome": "asteria:overworld/plains",
      "structure": "asteria:tree_oak",
      "spacing": 80,
      "chance": 0.54,
      "jitter": 24
    }
  ]
}
```

The first migrated roots are the existing Plains oak/willow groups plus small/medium/big/huge boulders, preserving their previous spacing/chance/jitter values while resolving them against the rebuilt biome, terrain, material, and generated-fluid owners.

### What is intentionally incomplete in Phase 6

The current `StructureField` remains a root-placement foundation, not the completed Structure graph planner. Remaining work includes:

- `StructureSet` expansion (`relativeTo`, min/max distance, separation, attempts, overlap rules) inside the same placement graph;
- connector-chain expansion and connector strength/distance behavior;
- extending the same priority/conflict/reservation arbitration over multi-piece Set/connector placement graphs rather than inventing a second resolver;
- preserving connected rivers, waterfalls, cavern entrances/tunnels, and other paths as generic Structures/connectors rather than creating a hydrology subsystem;
- migration of the remaining current biome/dimension Structure root authoring beyond the first Plains slice;
- final scalar/bounded-query seam/order validation for complete planned Structure graphs;
- exposing the finalized same placements to Phase 7 materialization and Phase 8 `/locate structure`.

Structure planning must remain independent of chunk materialization. A requesting chunk may ask which planned pieces intersect it, but chunk boundaries must never become Structure boundaries or planning inputs.

## Next concrete work

Continue **Phase 6 — Structures/features**, not chunk synthesis.

Required direction:

1. extend the existing `StructureField`; do not add a parallel planner;
2. integrate preserved `StructureSet` expansion into the same deterministic placement graph, including `relativeTo`, authored distances/separation/attempts, overlap rules, and complete graph bounds;
3. apply the existing root priority/conflict/reservation semantics to the resulting multi-piece placement intent;
4. integrate connector-chain expansion after Sets, preserving generic rivers/waterfalls/connectors and avoiding any hydrology subsystem;
5. migrate remaining authored Structure roots only through this owner;
6. validate that overlapping bounded requests observe the same complete logical placements regardless of request origin/order;
7. only after Phase 6 is complete move to Phase 7 `VoxelChunk` synthesis.

## Important non-regression rules

- No legacy worldgen implementation may be restored for convenience.
- No semantic generation region or chunk-owned biome/terrain/Structure truth.
- No separate land/ocean ownership model.
- No hydrology subsystem; rivers remain generic connected Structures/connectors.
- No duplicate biome map, terrain sampler, material sampler, generated-fluid sampler, Structure planner, or query resolver.
- Queries must remain pure, deterministic, world-coordinate anchored, and independent of generation/query order.
- No compatibility shims for deleted generation contracts unless explicitly requested.
- No `cargo test` in agent workflows/CI unless explicitly requested for the current task.
