# World Systems Rebuild — Implementation Status

This document tracks the current implementation state of the world-systems rebuild. The architecture and behavioral contract remain in [`world-systems-rebuild.md`](world-systems-rebuild.md); this file answers only: what is implemented now, what is intentionally incomplete, and what comes next.

Implementation branch: `world-systems-rebuild`

Code baseline recorded here: `3291594fefded1aa6c09dbffd88aea372cd277af` (`Fix generated fluid Clippy gate`), including `53e09fa0f087210f55dfbbad4a6009368b25569b` (`Add shallow swamp surface puddles`). Rust validation run `37260578857` completed successfully for that baseline. Terrain invariant validation remains available through the on-demand terrain debug path introduced before Phase 5.

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
| 5 — Surface/materials/generated fluids | In progress | Authoritative solid composition, deterministic surface patches, generated Ocean water, and shallow generated swamp puddles are implemented. Local lava and the generated-fluid-to-runtime-fluid frontier publication boundary remain pending. |
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
- `src/world/generator/generated_fluid.rs`
- `src/content/biome.rs` `surfaceLayers` and `surfaceLayers.patch`
- `src/content/dimension/types.rs` `generatedOcean` and `generatedSurfaceFluids`
- the current surface-biome JSON definitions under `data/dimensions/*/biomes/`
- `data/dimensions/overworld/dimension.json`

Current material/generated-fluid contract:

- `TerrainField` remains the sole owner of whether a voxel is solid or empty; material composition never recreates terrain occupancy;
- `GeneratedFluidField` is the one generated-fluid spatial-rule owner. It consumes immutable dimension authoring, deterministic world-space entropy, and authoritative primary surface-biome samples for both Ocean and local surface-fluid placement;
- `TerrainField` consumes only the shallow local-fluid cut depth from that same `GeneratedFluidField`, so a puddle removes terrain through the authoritative density field instead of placing fluid inside a still-solid voxel;
- `MaterialField` owns generated solid block composition and exposes initial generated-fluid queries by consuming `TerrainField`, `BiomeLayout`, and the shared `GeneratedFluidField` rather than re-resolving fluid placement;
- `MaterialQueries::solid_block_at(x, y, z)` returns no block for empty terrain and the authored solid block for occupied terrain;
- `MaterialQueries::sample_solid_volume(...)` provides dense unit-step XYZ solid sampling for later chunk synthesis without materializing a `VoxelChunk`;
- `MaterialQueries::generated_fluid_at(x, y, z)` returns the initial generated fluid at an empty world-space voxel;
- `MaterialQueries::sample_generated_fluid_volume(...)` provides dense unit-step XYZ generated-fluid sampling through the same Phase 5 ownership chain;
- discrete solid/fluid ownership uses the authoritative primary surface biome from `BiomeQueries`, so no second biome-boundary or Ocean mask exists;
- base terrain layers measure depth from the authoritative base surface;
- additive terrain above the base surface, including floating formations, resolves its local exposed top with a bounded upward final-density scan capped by the authored finite material-layer depth, then applies the same surface/subsurface/core profile;
- deep cave walls/floors therefore remain core material rather than incorrectly becoming surface soil merely because a cave exposes them;
- batch solid/fluid composition reuses authoritative terrain-column, density-volume, and biome-area results, preserving scalar/batch ownership and world-coordinate semantics;
- every currently authored surface biome in Overworld and Umbral has an explicit solid `surfaceLayers` profile; `MaterialField` refuses to construct a dimension whose participating surface biome lacks one;
- finite material layers may optionally author deterministic world-space patch replacements;
- patch placement uses seed/domain-addressed world-space lattice cells with deterministic presence, jitter, circular extent, stable overlap tie-breaking, and alternate-block selection;
- scalar and bounded solid-material queries call the same patch resolver, so chunk/request origin cannot change the selected material;
- swamp topsoil interleaves `grass_block`, `dirt`, and `mud`, while its finite dirt subsurface may transition into deterministic mud patches;
- the Overworld dimension explicitly authors `generatedOcean.biome = asteria:overworld/ocean` and `generatedOcean.fluid = asteria:water`;
- generated Ocean water exists only when the authoritative primary surface biome matches that authored Ocean biome, the final terrain density is empty, and the voxel lies strictly above the authoritative base Ocean floor and at or below dimension `seaLevel`;
- empty cave/overhang volume below the generated Ocean floor remains dry because generated Ocean fill never applies at or below `base_surface_at(x, z)`;
- the Overworld also authors a deterministic `generatedSurfaceFluids` water rule for `asteria:overworld/swamp`;
- swamp puddle placement is world-coordinate anchored and seed/domain deterministic, using authored spacing/radius/jitter/chance rather than chunk/request-local randomness;
- the current swamp puddle depth is one voxel: `TerrainField` carves the former top solid voxel, `surface_at` resolves the solid puddle bed below it, and `GeneratedFluidField` fills exactly that removed voxel with `asteria:water`;
- this makes water genuinely interleave with grass/dirt/mud at the swamp surface instead of floating one block above an unchanged solid surface;
- generated fluid is initial world formation only. Runtime propagation/frontier scheduling remains a later explicit boundary and generation does not bulk-schedule generated Ocean or puddle voxels.

Current solid layer authoring shape:

```json
{
  "surfaceLayers": [
    {
      "block": "asteria:grass_block",
      "depth": 1,
      "patch": {
        "spacing": 14,
        "radius": 6,
        "jitter": 2,
        "chance": 0.9,
        "blocks": [
          "asteria:dirt",
          "asteria:mud"
        ]
      }
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

Current generated-fluid authoring shape:

```json
{
  "seaLevel": 90,
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
    }
  ]
}
```

Finite solid entries require a positive `depth`. The final entry is the depthless core layer. The total finite authored depth is bounded. Patches are optional and may only replace finite layers; they remain deterministic spatial material variation rather than a second terrain or biome field. Wasteland is explicitly dirt/gravel/stone, mountain/gorge profiles remain stone-dominant, Ocean floor uses sand/gravel/stone, and the Umbral profiles preserve their authored solid identities.

### What is not implemented yet in Phase 5

Solid layers, deterministic solid surface patches, authoritative generated Ocean fill, and shallow generated swamp puddles are implemented. Remaining work includes:

- local generated fluids such as volcano crater lava where authored;
- the explicit handoff from generated-fluid initial state to runtime dynamic-fluid frontier scheduling.

Generated fluids must remain initial world formation, not a hydrology subsystem. Local generated fluids must reuse authoritative biome/terrain/material facts rather than creating independent spatial ownership.

## Next concrete work

Continue **Phase 5**, not chunk synthesis.

Required direction:

1. add the next authored local generated-fluid formation, starting with volcano crater lava if the current volcano content requires it, through the existing `GeneratedFluidField` rather than a new subsystem;
2. keep scalar and dense bounded generated-fluid queries deterministic and world-coordinate anchored, reusing authoritative biome/terrain/material results rather than introducing another resolver;
3. define the generated-fluid-to-runtime-fluid frontier boundary so initial generation does not bulk-schedule every generated fluid voxel, while exposed empty neighbors can seed runtime continuation when materialization is introduced;
4. preserve the Ocean rule that subterranean cave/overhang voids below the generated floor stay dry;
5. preserve shallow surface-fluid cuts as authoritative Terrain density changes rather than material-only overrides;
6. remain pre-`VoxelChunk` until Phase 7.

## Important non-regression rules

- No legacy worldgen implementation may be restored for convenience.
- No semantic generation region or chunk-owned biome/terrain truth.
- No separate land/ocean ownership model.
- No hydrology subsystem; rivers remain generic connected Structures/connectors.
- No duplicated biome map, terrain sampler, material sampler, generated-fluid sampler, or query resolver.
- No compatibility shims for deleted generation contracts unless explicitly requested.
- No `cargo test` in agent workflows/CI unless explicitly requested for the current task.
