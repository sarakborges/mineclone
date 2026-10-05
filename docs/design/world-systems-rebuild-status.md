# World Systems Rebuild — Implementation Status

This document tracks the current implementation state of the world-systems rebuild. The architecture and behavioral contract remain in [`world-systems-rebuild.md`](world-systems-rebuild.md); this file answers only: what is implemented now, what is intentionally incomplete, and what comes next.

Implementation branch: `world-systems-rebuild`

Code baseline recorded here: `61ec05c6d4de1f075155300fcd4b2c16e3a8f706` (`Add authoritative solid material composition`). Rust validation run `37257235275` completed successfully for that baseline. Terrain invariant validation remains available through the on-demand terrain debug path introduced before Phase 5.

## Validation policy

Repository validation follows root `AGENTS.md`.

- Do not run or add `cargo test` unless the user explicitly requests that command for the current task.
- Default Rust validation is Clippy with `-D warnings`, `cargo check --locked`, and applicable content/localization/GLB audits.
- The rebuild plan may describe behavioral invariants and regression coverage, but that does not override the repository command policy.
- Visual/debug validation is an on-demand consumer capability. It is not a permanent CI fixture-rendering loop.

## Phase status

| Phase | Status | Current state |
| --- | --- | --- |
| 0 — Impact audit | Complete | External consumers and required capability boundaries are documented in the rebuild plan. |
| 1 — Cleanup | Complete | Legacy biome/world-generation/loading ownership and transitional biome presentation bridges were removed. Preserved runtime systems remain separate from the new generator. |
| 2 — Generation foundation | Complete | New generation foundation owns world-space coordinates, deterministic entropy/domains, immutable dimension snapshot state, direct far-coordinate queries, and scalar/batch primitives. |
| 3 — Biome Layout | Complete for current authored content | New surface layout, biome queries, influences, search, `regionSize`, `cannotBorder`, and authoritative biome-map rendering exist. No volume-biome content is currently authored, so the volume query returns no override and effective biome falls back to surface ownership. |
| 4 — Terrain | Complete for current authored content | Continuous 2D base terrain, authoritative final 3D density, bounded caves, authored floating masses, bounded effective-surface resolution, scalar/batch queries, and the terrain debug renderer with built-in seam/query validation are implemented. No currently authored terrain requires another true-3D form. |
| 5 — Surface/materials/generated fluids | In progress | One authoritative solid `MaterialField`/`MaterialQueries` now composes biome-authored surface/subsurface/core layers from `TerrainField` solidity. Scalar and dense XYZ solid-material queries exist for every current surface biome. Surface patch variants and generated natural fluids/Ocean fill are still pending. |
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

The viewer consumes `BiomeQueries`, the same source used by generation. It supports deterministic PNG output, stable biome colors, center/scale/output resolution, boundary and influence rendering, and JSON metadata. It must remain a consumer of the authoritative layout rather than becoming another resolver.

## Phase 4 — completed terrain for current authored content

Authoritative implementation:

- `src/world/generator/terrain.rs`
- `src/world/generator/terrain_debug.rs`
- `src/content/biome.rs` `surfaceTerrain` and `terrain3d`

Implemented terrain contract:

- `TerrainQueries::base_surface_at(x, z)` is the continuous authoritative 2D base surface and is unchanged by true-3D contributions;
- `TerrainQueries::density_at(x, y, z)` is the one authoritative final terrain-density answer;
- final density composes base solidity, bounded subtractive cave carving, and biome-authored bounded additive floating formations inside the same `TerrainField`;
- cave noise is deterministic, anchored in world coordinates, and bounded to a finite depth band below the base surface;
- `terrain3d.floatingFormation` authors finite absolute vertical bounds and shape parameters for additive floating masses;
- the current Overworld `floating_islands` biome authors its floating formation in world Y `200..280`;
- floating formation shape and biome-edge retreat consume authoritative biome influence weights rather than resolving biome boundaries independently;
- `TerrainQueries::surface_at(x, z)` finds the highest effective solid crossing from the base result plus finite candidate bounds exposed by active 3D contributions, never by scanning the full world height;
- bounded surface-grid queries use the same effective-surface resolver as scalar queries;
- `TerrainQueries::sample_density_volume(...)` provides dense XYZ sampling while reusing authoritative biome/base-column work;
- noise/lattices remain world-coordinate anchored, preserving the same fact when sampled alone or from overlapping bounded requests;
- every current surface biome has a `surfaceTerrain` profile;
- no current authored biome requires a separate overhang form or volume-biome terrain effect.

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

### Terrain visual/debug capability

`src/world/generator/terrain_debug.rs` is a pure consumer of `TerrainQueries` and does not own or reimplement terrain semantics.

`--terrain-debug` can render:

- an X/Z effective-surface image;
- an X/Y final-density slice at a selected Z;
- JSON metadata containing sampled ranges and density classifications;
- a built-in validation report for scalar/batch equivalence, overlapping bounded-request seams, repeated density sampling after a differently-originated request, and effective-surface crossing consistency.

The density view distinguishes base solid, cave carve, additive mass, the base crossing, and the effective additive crossing. The surface view highlights columns whose effective top rises above the base surface. The CLI writes the visual/JSON diagnostics first and then fails explicitly if any built-in terrain-query invariant reports a mismatch. This satisfies the rebuild gate that deterministic seam/query validation and visual/debug terrain inspection be available before material composition.

Temporary CI fixture-rendering probes were deliberately removed because linking the full dynamically configured game binary inside every validation run was a blocking execution-environment cost rather than a terrain ownership requirement. The normal CI remains limited to the repository validation gate; debug rendering and its invariant checks stay on-demand.

### Deferred extensions

These do not block Phase 4 for the content currently authored:

- future volume-biome terrain effects, because no volume-biome terrain content exists yet;
- additional overhang/additive forms if future biome authoring explicitly requires them.

Any such extension must compose into the same `TerrainField`, expose bounded candidates where effective-column queries need them, and preserve scalar/batch/world-coordinate equivalence.

## Phase 5 — surface/material composition in progress

Authoritative implementation currently being built:

- `src/world/generator/material.rs`
- `src/content/biome.rs` `surfaceLayers`
- the current surface-biome JSON definitions under `data/dimensions/*/biomes/`

Current solid-material contract:

- `TerrainField` remains the sole owner of whether a voxel is solid or empty; material composition never recreates terrain occupancy;
- `MaterialField` owns generated solid block identity and is constructed from the same immutable generation snapshot, authoritative `BiomeLayout`, and authoritative `TerrainField`;
- `MaterialQueries::solid_block_at(x, y, z)` returns no block for empty terrain and the authored solid block for occupied terrain;
- `MaterialQueries::sample_solid_volume(...)` provides dense unit-step XYZ sampling for later chunk synthesis without materializing a `VoxelChunk`;
- discrete solid-material ownership uses the authoritative primary surface biome from `BiomeQueries`, so no second biome-boundary resolver exists;
- base terrain layers measure depth from the authoritative base surface;
- additive terrain above the base surface, including floating formations, resolves its local exposed top with a bounded upward final-density scan capped by the authored finite material-layer depth, then applies the same surface/subsurface/core profile;
- deep cave walls/floors therefore remain core material rather than incorrectly becoming surface soil merely because a cave exposes them;
- batch composition reuses one terrain density volume, one terrain column area, and one biome area for the bounded request, preserving the scalar/batch ownership model;
- every currently authored surface biome in Overworld and Umbral has an explicit solid `surfaceLayers` profile; `MaterialField` refuses to construct a dimension whose participating surface biome lacks one.

Current solid layer authoring shape:

```json
{
  "surfaceLayers": [
    {
      "block": "asteria:grass_block",
      "depth": 1
    },
    {
      "block": "asteria:dirt",
      "depth": 4
    },
    {
      "block": "asteria:stone"
    }
  ]
}
```

Finite entries require a positive `depth`. The final entry is the depthless core layer. The total finite authored depth is bounded. Wasteland is explicitly dirt/gravel/stone, mountain/gorge profiles remain stone-dominant, Ocean floor uses sand/gravel/stone, and the Umbral profiles preserve their authored solid identities.

### What is not implemented yet in Phase 5

The solid base profile is only the first Phase 5 slice. Remaining work includes:

- deterministic surface-patch/alternate material rules, including the swamp grass/dirt/mud pattern;
- generated swamp surface water/puddles;
- authoritative generated Ocean water fill between the Ocean floor and dimension `seaLevel`;
- local generated fluids such as volcano crater lava where authored;
- the explicit handoff from generated-fluid initial state to runtime dynamic-fluid frontier scheduling.

Generated fluids must remain initial world formation, not a hydrology subsystem. Ocean fill must depend on primary Ocean ownership and exposed Ocean volume and must not flood arbitrary caves/overhang voids below the Ocean floor.

## Next concrete work

Continue **Phase 5**, not chunk synthesis.

Required direction:

1. add deterministic authored surface-patch rules for discrete top-layer variants, starting with swamp grass/dirt/mud while reusing the same biome/material owner;
2. add generated-fluid composition to the same Phase 5 owner, starting with Ocean exposed fill from its floor to dimension `seaLevel` only where primary surface ownership is Ocean;
3. keep subterranean cave/overhang voids below the Ocean floor dry unless an explicit local generated-fluid feature owns them;
4. add local generated fluids such as swamp puddles and volcano crater lava through explicit authored rules, without introducing hydrology ownership;
5. define the generated-fluid-to-runtime-fluid frontier boundary so initial generation does not bulk-schedule every generated fluid voxel;
6. preserve scalar/batch/world-coordinate equivalence and remain pre-`VoxelChunk` until Phase 7.

## Important non-regression rules

- No legacy worldgen implementation may be restored for convenience.
- No semantic generation region or chunk-owned biome/terrain truth.
- No separate land/ocean ownership model.
- No hydrology subsystem; rivers remain generic connected Structures/connectors.
- No duplicated biome map, terrain sampler, material sampler, or query resolver.
- No compatibility shims for deleted generation contracts unless explicitly requested.
- No `cargo test` in agent workflows/CI unless explicitly requested for the current task.
