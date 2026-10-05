# World Systems Rebuild — Implementation Status

This document tracks the current implementation state of the world-systems rebuild. The architecture and behavioral contract remain in [`world-systems-rebuild.md`](world-systems-rebuild.md); this file records only what is implemented now, what remains, and the next concrete cut.

Implementation branch: `world-systems-rebuild`

Code baseline recorded here: `6231b58e91d62df290a392de89e87bdd760b6f54` (`Add biome-margin Structure roots`), including connector graph integration rooted at `86143c68910c16e8479f13020933c2348c82576e` (`Add deterministic connector graph expansion`) and the complete simple-root migration at `c3c07e0604d08514e6b881a62b772d1f9d152d05` (`Migrate remaining surface structure roots`). Rust validation run `37310832213` completed successfully for that baseline. Terrain visual/invariant validation remains available through the on-demand terrain debug path.

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
| 6 — Structures/features | In progress | `StructureField` owns deterministic root placement/query, conflict arbitration, multi-piece `StructureSet` expansion, connector-chain expansion, all current simple surface roots, and generic biome-margin roots. Ocean margins now seed river-mouth graphs; remaining connected-feature roots plus final placement-graph seam/order validation are pending. |
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

## Phase 6 — Structures/features in progress

Authoritative generation-side implementation:

- `src/world/generator/structure.rs`
- `src/world/generator/structure/connector.rs`
- `src/world/generator/structure_set.rs`
- preserved generic Structure authoring/runtime definitions in `src/content/structure.rs`
- preserved restriction definitions in `src/content/structure_rules.rs`
- preserved composition definitions in `src/content/structure_set.rs`
- `src/content/dimension/types.rs` `generatedSurfaceStructures`
- `data/dimensions/overworld/dimension.json`

### Implemented Structure graph foundation

`StructureField` is the single generation-side owner for deterministic Structure placement. `ConnectorGraph` is an internal compiled capability of that owner, not a parallel planner. Runtime `/place structure` remains a manual mutation path and is not the generation owner.

Implemented now:

- root placements use deterministic seed/domain-addressed world-space lattice cells with authored `spacing`, `chance`, and `jitter`;
- root placement authoring now supports `placement = biomeInterior` (default) and `placement = biomeMargin` without creating another Structure planner;
- `biomeMargin` keeps the same deterministic world-space lattice but uses its jittered cell point only as a selection hint: the root resolver samples authoritative primary-biome facts inside that cell, refines a detected cardinal boundary to adjacent inside/outside voxels, and chooses the exterior boundary voxel nearest that hint;
- `biomeMargin` orientation is derived from the authoritative inside-to-outside cardinal normal, so an authored Structure's `front` points away from the referenced biome; it does not independently infer another biome boundary field;
- a generated-fluid-backed biome margin can use the existing dimension `seaLevel` as the root ground plane only when the target-side voxel actually contains generated fluid there; this keeps an Ocean mouth at the generated water surface instead of fitting it to the Ocean floor, while connector children return to ordinary terrain ground fitting;
- `biomeMargin` currently resolves concrete Structures/Structure groups only and requires every possible member to support all four horizontal rotations; StructureSets remain on ordinary authored roots until a real content requirement justifies a set-level margin contract;
- a root reference may resolve to a concrete Structure, Structure group, or `StructureSet`; all referenced definitions/variants are frozen into the immutable generator snapshot;
- authored Structure rotation and group variation are deterministic;
- ordinary biome-interior root candidates require the authoritative primary surface biome instead of resolving biome boundaries independently;
- ground fitting consumes `TerrainQueries::surface_at` and the existing Structure support/full-footprint rules;
- authored min/max slope, min/max Y, `groundBlocks`, `requiredBiomeCoverage`, dry-ground, fluid-forbid, and block/fluid proximity constraints consume authoritative biome/terrain/material/generated-fluid queries;
- root arbitration is intent-based: a candidate is rejected whenever a directly overlapping higher-ranked conflicting intent exists, regardless of whether that higher-ranked intent would itself later survive another conflict;
- ranking is deterministic: authored `priority` descending, then placement reference, biome id, logical root anchor, and placement Y;
- `reserveSpace` is directional and shared `conflictGroups` are symmetric after deterministic ranking selects the higher intent;
- arbitration uses complete 3D logical placement bounds rather than request/chunk bounds;
- `StructureSet` elements deterministically apply authored chance, count min/max, group variation, rotation, `relativeTo`, distances, separation, attempts, and overlap rules;
- required set elements reject the whole logical set when their authored minimum count cannot be placed;
- each resolved set piece is ground-fit and restriction-checked through the same authoritative Terrain/Biome/Material queries as a direct root;
- set-level `priority`, `conflictGroups`, and `reserveSpace` arbitrate the whole multi-piece logical placement, not each piece independently;
- `ConnectorGraph` freezes concrete Structure and Structure-group connector targets and sorts group members by Structure id before deterministic selection;
- connector output faces are rotated into world space and attach only to compatible opposite-face inputs, reusing the authored input-attachment contract and supported child rotations;
- connector target variant, attachment, and min/max-distance choice are deterministic functions of immutable generation entropy plus parent world position/identity;
- connector `strength` and `strengthLossOnEachLoop` propagate through breadth-first expansion; a child continues only while remaining strength is positive;
- all roots of one direct/Set logical placement share one connector occupancy set, so connector children cannot overlap persistent voxels of sibling/root pieces or previously accepted children inside that graph;
- connected children that require ground fit use the authoritative terrain surface; all connected children pass through the same Structure restrictions used by direct and Set pieces;
- connector expansion happens before final placement bounds and arbitration, so root/Set `priority`, `conflictGroups`, and `reserveSpace` arbitrate the complete connector-expanded logical graph;
- query candidate padding uses connector-expanded finite theoretical bounds, while arbitration uses the actual resolved union bounds;
- recursive connector cycles that return to the same Structure/rotation/remaining-strength state are rejected during bound compilation; recursive authored chains must lose strength so query reach remains finite instead of relying on an arbitrary depth cap;
- `StructureQueries::placements_intersecting(...)` plans/arbitrates complete logical graphs first and only then returns surviving pieces intersecting the requested rectangle;
- a multi-piece Set/connector graph can therefore cross a future chunk/request boundary without that boundary changing its composition or arbitration result;
- `StructureQueries::find_nearest(...)` measures distance to the logical root anchor and reuses the same accepted placement graph; the representative returned piece retains that logical `placement_anchor`;
- runtime world installation freezes both `StructureRegistry` and `StructureSetRegistry` into the generator-side owner;
- all current simple surface-biome roots from the preserved Overworld authoring have been migrated into `generatedSurfaceStructures`: Plains oak/willow and four boulder sizes; Swamp willow plus small boulders; Enchanted Forest heart, enchanted trees, and small boulders; Wasteland four boulder sizes; Mountains four boulder sizes; Gorge small/medium/big boulders; Alps medium/big/huge boulders; and Mountain Belt four boulder sizes;
- those migrated roots preserve their historical spacing/chance/jitter values while resolving exclusively through the rebuilt `StructureField`;
- Arctic, Desert, Ocean, Volcano, and Floating Islands had no simple biome-interior surface roots in the preserved authoring and therefore do not receive invented interior roots;
- the current Umbral dimension authors no Structure roots, so none are restored implicitly;
- Overworld now authors `asteria:river_ocean_mouth` as a `biomeMargin` root of `asteria:overworld/ocean` with current tuning `spacing = 192`, `chance = 0.55`, `jitter = 48`; its connector expands ordinary `asteria:river_segment` Structures through the same generic connector graph and arbitration path.

Rivers still have no dedicated source graph, downstream rasterizer, or hydrology owner. Ocean mouths are ordinary margin-relative Structure roots, lake Structures already carry ordinary river-segment output connectors, and every resulting connected piece remains part of the same `StructureField` logical graph.

### What is intentionally incomplete in Phase 6

The current `StructureField` owns direct roots, StructureSets, connector-expanded logical graphs, all current simple lattice/biome roots, and generic biome-margin roots, but Phase 6 is not complete. Remaining work includes:

- migration/authoring of the remaining connected-feature roots required by current content, including lake roots and any waterfall/cavern entrance or tunnel roots that should enter generation;
- extending world-fact-relative root placement only when those concrete features require a fact other than biome margins; do not create feature-specific planners;
- final scalar/bounded-query seam/order validation for complete planned Structure graphs, including connector chains rooted at biome margins;
- exposing the finalized same placements to Phase 7 materialization and Phase 8 `/locate structure`.

Structure planning must remain independent of chunk materialization. A requesting chunk may ask which planned pieces intersect it, but chunk boundaries must never become Structure boundaries or planning inputs.

## Next concrete work

Continue **Phase 6 — Structures/features**, not chunk synthesis.

Required direction:

1. inspect the remaining connected Structure content and migrate/author only the roots actually required for lakes, waterfalls, cavern entrances/tunnels, and similar features;
2. reuse ordinary biome-interior roots, `biomeMargin`, StructureSets, and connectors wherever those contracts already express the placement; add another generic world-fact-relative placement mode only when a real remaining feature cannot be represented by them;
3. keep rivers/waterfalls generic Structure/connector graphs and do not introduce a hydrology subsystem;
4. keep connector-expanded graph arbitration under the existing root/Set priority/conflict/reservation owner;
5. add final deterministic seam/order validation proving overlapping bounded requests observe the same complete logical graph regardless of request origin/order;
6. only after Phase 6 is complete move to Phase 7 `VoxelChunk` synthesis.

## Important non-regression rules

- No legacy worldgen implementation may be restored for convenience.
- No semantic generation region or chunk-owned biome/terrain/Structure truth.
- No separate land/ocean ownership model.
- No hydrology subsystem; rivers remain generic connected Structures/connectors.
- No duplicate biome map, terrain sampler, material sampler, generated-fluid sampler, Structure planner, or query resolver.
- Queries must remain pure, deterministic, world-coordinate anchored, and independent of generation/query order.
- No compatibility shims for deleted generation contracts unless explicitly requested.
- No `cargo test` in agent workflows/CI unless explicitly requested for the current task.
