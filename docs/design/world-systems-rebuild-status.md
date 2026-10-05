# World Systems Rebuild — Implementation Status

This document tracks the current implementation state of the world-systems rebuild. The architecture and behavioral contract remain in [`world-systems-rebuild.md`](world-systems-rebuild.md); this file records only what is implemented now, what remains, and the next concrete cut.

Implementation branch: `world-systems-rebuild`

Code baseline recorded here: `4c6138db22b49e73049f9c7a7bbc92c82a3b7964` (`Keep streaming system private`). Rust validation run `37333455955` completed successfully for this baseline, including localization/content/GLB audits, the Structure query contract audit, the chunk materializer contract audit, the chunk seam audit, the streaming materialization contract audit, Clippy with `-D warnings`, and `cargo check --locked`. Terrain/Structure debug capabilities remain available on demand and are not permanent full-binary CI probes.

## Validation policy

Repository validation follows root `AGENTS.md`.

- Do not run or add `cargo test` unless the user explicitly requests that command for the current task.
- Default Rust validation is Clippy with `-D warnings`, `cargo check --locked`, and applicable content/localization/GLB audits.
- `tools/check_structure_query_contract.py` is the lightweight CI gate for Structure bounded-query/order invariants; it reads the authoritative owner shape, verifies current authored probe families, and exercises overlap/order/nearest contracts without linking or running the game binary.
- `tools/check_chunk_materializer_contract.py` is the lightweight Phase 7 composition gate; it verifies dense semantic sampling occurs before runtime composition, then guards block → generated-fluid → planned-Structure → generated-fluid-frontier ordering without introducing `VoxelWorld` ownership.
- `tools/check_chunk_seams.py` is the lightweight Phase 7 seam/order gate; it freezes scalar/batch owner shape, world/chunk partitioning (including negative coordinates), Structure/`clearAbove` clipping, attachment-only world-space projection, generated-fluid frontier behavior, and reordered-neighbor semantic fixtures without linking/running the game binary.
- `tools/check_streaming_materialization_contract.py` is the lightweight Phase 8 streaming gate; it guards generator-owned selection/materialization, stale async-result rejection, restored-vs-new publication, resident-before-frontier ordering, sparse generated-fluid frontier handoff, and deterministic chunk-priority tie-breaks without executing the game binary.
- `tools/check_streaming_presentation_contract.py` is the lightweight Phase 8 presentation gate; it guards resident publication before direct-light activation, presentation-selection sync, remesh-owned initial mesh work, and neighbor halo catch-up without introducing another meshing owner.
- `tools/check_locate_generator_contract.py` is the lightweight Phase 8 query-consumer gate; it requires `/locate` to use `WorldGenerator` biome/terrain/Structure capabilities and rejects runtime chunk scans, raw seed access, materialization, and owner-internal dependencies.
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
| 6 — Structures/features | Complete for current authored content | `StructureField` owns deterministic root placement/query, conflict arbitration, multi-piece `StructureSet` expansion, connector-chain expansion, all current simple surface roots, biome-margin roots, lake/waterfall connected roots, and a lightweight CI seam/order/nearest contract gate. |
| 7 — Chunk synthesis | Complete for current authored content | `ChunkMaterializer` deterministically composes dense solid/material results, generated fluids, already-planned Structure pieces, attachment-only payloads, and sparse generated-fluid frontier targets into one `VoxelChunk`; lightweight adjacent-chunk seam/order validation is green. |
| 8 — Consumer integration | In progress | Streaming materialization, restored/new publication, generated-fluid frontier handoff, resident lighting/presentation activation, and `/locate biome`/`/locate structure` are generator-backed. Spawn/warp/portal and remaining generated-world consumers still need migration/audit. |
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
- `enqueue_generated_fluid_frontier(...)` is the explicit runtime handoff for exposed continuation targets only.

## Phase 6 — Structures/features complete for current authored content

Authoritative generation-side implementation:

- `src/world/generator/structure.rs`
- `src/world/generator/structure/connector.rs`
- `src/world/generator/structure_set.rs`
- preserved generic Structure authoring/runtime definitions in `src/content/structure.rs`
- preserved restriction definitions in `src/content/structure_rules.rs`
- preserved composition definitions in `src/content/structure_set.rs`
- `src/content/dimension/types.rs` `generatedSurfaceStructures`
- `data/dimensions/overworld/dimension.json`
- `tools/check_structure_query_contract.py` for the lightweight CI request-window/order contract gate

### Implemented Structure graph foundation

`StructureField` is the single generation-side owner for deterministic Structure placement. `ConnectorGraph` is an internal compiled capability of that owner, not a parallel planner. Runtime `/place structure` remains a manual mutation path and is not the generation owner.

Implemented now:

- root placements use deterministic seed/domain-addressed world-space lattice cells with authored `spacing`, `chance`, and `jitter`;
- root placement authoring supports `placement = biomeInterior` (default) and `placement = biomeMargin` without creating another Structure planner;
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
- `StructureQueries::placements_intersecting(...)` plans/arbitates complete logical graphs first and only then returns surviving pieces intersecting the requested rectangle;
- a multi-piece Set/connector graph can therefore cross a future chunk/request boundary without that boundary changing its composition or arbitration result;
- `StructureQueries::find_nearest(...)` measures distance to the logical root anchor and reuses the same accepted placement graph; the representative returned piece retains that logical `placement_anchor`;
- `StructureQueries::find_nearest_variant(...)` applies a concrete direct-root variation filter inside the same accepted nearest-root search, so `/locate` does not recreate Structure selection or scan materialized chunks;
- runtime world installation freezes both `StructureRegistry` and `StructureSetRegistry` into the generator-side owner;
- all current simple surface-biome roots from the preserved Overworld authoring have been migrated into `generatedSurfaceStructures`: Plains oak/willow and four boulder sizes; Swamp willow plus small boulders; Enchanted Forest heart, enchanted trees, and small boulders; Wasteland four boulder sizes; Mountains four boulder sizes; Gorge small/medium/big boulders; Alps medium/big/huge boulders; and Mountain Belt four boulder sizes;
- those migrated roots preserve their historical spacing/chance/jitter values while resolving exclusively through the rebuilt `StructureField`;
- Arctic, Desert, Ocean, Volcano, and Floating Islands had no simple biome-interior surface roots in the preserved authoring and therefore do not receive invented interior roots;
- the current Umbral dimension authors no Structure roots, so none are restored implicitly;
- Overworld authors `asteria:river_ocean_mouth` as a `biomeMargin` root of `asteria:overworld/ocean` with `spacing = 192`, `chance = 0.55`, `jitter = 48`; its connector expands ordinary `asteria:river_segment` Structures through the same generic connector graph and arbitration path;
- Plains now authors `asteria:lake` as an ordinary biome-interior root with `spacing = 256`, `chance = 0.5`, `jitter = 64`; the lake group's existing output connector starts an ordinary `asteria:river_segment` chain and the lake's authored `reserveSpace`/`surface_water_body` conflict semantics remain authoritative;
- Mountains now authors `asteria:mountain_waterfall` with `spacing = 192`, `chance = 0.45`, `jitter = 48`; Alps uses `spacing = 224`, `chance = 0.4`, `jitter = 56`; Mountain Belt uses `spacing = 224`, `chance = 0.32`, `jitter = 56`;
- mountain-waterfall roots use their existing `minSlope = 3` / `maxSlope = 8` terrain restriction rather than a feature-specific slope locator; their existing connector creates `asteria:mountain_pond`, whose existing output connector then expands ordinary `asteria:river_segment` pieces;
- no independent root is authored for `mountain_pond`, `river_segment`, or `river_lake`; these remain connector content rather than becoming parallel world-feature planners;
- the preserved historical Caverns authoring used `asteria:cavern_entrance_start` only through volume placement (`chance = 0.25`). The rebuild currently authors no volume-biome field (`data/dimensions/overworld/biomes/caverns.json` is intentionally empty beyond its id), so cavern entrances/tunnels are not misrepresented as surface roots. They remain preserved Structure/connector content until volume-biome authoring/capability is intentionally introduced;
- the lightweight CI contract audit verifies that bounded requests still collect world-space candidates, arbitrate complete logical placements before request filtering, use complete logical bounds, sort output deterministically, and keep `StructureField` free of interior mutable query state;
- the same audit verifies current authoring still contains the ordinary-root, StructureSet, biome-margin river-mouth, lake, and mountain-waterfall probe families, then exercises overlapping A→B→B→A request windows across representative ordinary root, multi-piece Set, mouth→river, lake→river, and waterfall→pond→river graphs;
- the audit also guards `find_nearest(...)` source shape and validates nearest selection by logical root anchor with deterministic equal-distance tie-breaking rather than representative/connector-piece origin.

Rivers still have no dedicated source graph, downstream rasterizer, or hydrology owner. Ocean mouths, Plains lakes, and mountain waterfall/pond graphs are ordinary Structure roots/connectors, and every resulting connected piece remains part of the same `StructureField` logical graph.

### Phase 6 validation boundary

For the currently authored surface world, root migration/authoring and the deterministic bounded-query/order contract gate are complete. The full `--structure-debug` seed/content probe remains available for manual/on-demand diagnosis, but it is intentionally not executed in normal CI because linking/running the full game binary made CI stall without adding a distinct authoritative owner.

Cavern entrance/tunnel Structures are preserved but are not a current Phase 6 surface-root requirement. If volume-biome authoring returns later, their historical volume-root semantics must be implemented against the authoritative volume-biome capability rather than approximated with a surface lattice.

Structure planning remains independent of chunk materialization. A requesting chunk may ask which planned pieces intersect it, but chunk boundaries must never become Structure boundaries or planning inputs.

## Phase 7 — Chunk synthesis complete for current authored content

Authoritative materialization-side implementation:

- `src/world/generator/chunk.rs`
- `src/world/generator.rs` for the `WorldGenerator::materialize_chunk(...)` capability boundary
- `src/world/mod.rs` for immutable runtime-content installation
- `tools/check_chunk_materializer_contract.py` for the lightweight composition-order/ownership gate
- `tools/check_chunk_seams.py` for lightweight adjacent-chunk seam/order fixtures

Implemented now:

- `ChunkMaterializer` is the single adapter from semantic generator owners to one runtime `VoxelChunk`; the chunk remains materialized storage and does not become the owner of biome, terrain, material, fluid, or Structure generation facts;
- runtime installation freezes `BlockRegistry` and `FluidRegistry` only as encoding dictionaries for already-authoritative generated identities; they do not participate in spatial generation decisions;
- one chunk request resolves dense solid/material and generated-fluid volumes before mutating runtime chunk storage;
- solids are encoded first with deterministic world-position texture rotation;
- generated fluids are then encoded as initial source cells and assert that they do not overlap solid generated terrain;
- already-planned `StructureField` placements intersecting the chunk are queried after base composition and rasterized without replanning at chunk boundaries;
- Structure replacement semantics preserve the existing contract: `AirOnly` cannot replace base block/fluid occupancy or earlier claimed Structure content, while `Terrain` may replace base generated content but not earlier claimed Structure voxels;
- Structure block, fluid, clear, `clearAbove`, surface-layer, object, orientation, fluid-displacement, and attachment payloads are materialized through existing generic Structure definitions;
- cross-chunk Structure graphs are still planned globally by `StructureField`; the materializer only clips each already-resolved piece to the requested chunk volume;
- attachment-only markers no longer depend on the chunk containing the marker. Their support projection is resolved in world space against authoritative generated solid material, selects the highest valid support within authored `maxSlope`, and only the chunk containing that one support materializes the layer/object payload;
- the materializer's vertical Structure filter includes the possible attachment rise so a marker just below a section boundary may correctly project into the section above without requiring that marker voxel itself to be inside the target chunk;
- attachment payload application still verifies the chosen support block survived local Structure composition; generated-fluid checks outside the current chunk use scalar world-space material queries rather than neighboring residency;
- generated-fluid frontier candidates are collected only after Structure composition, so Structure displacement/clears affect the handoff exactly as materialized;
- frontier publication remains sparse: only exposed continuation targets are returned, never every generated fluid voxel;
- frontier targets outside the requested chunk remain potential until runtime revalidation sees neighboring materialized state;
- chunk synthesis is independent of `VoxelWorld`, streaming residency, neighboring chunk request order, and prior materialization history;
- the materializer contract audit freezes dense-sampling-before-composition and block → fluid → Structure → frontier order without linking/running the game binary;
- the seam audit verifies adjacent world/chunk partitioning, scalar/batch owner shape, Structure/`clearAbove` clipping, one-support attachment projection across vertical seams, inside-vs-cross-chunk fluid-frontier semantics, and repeated/reordered neighboring request fixtures.

### Phase 7 validation boundary

For the currently authored world-generation stack, the materialization ownership/composition contract and lightweight adjacent-chunk seam/order gate are complete. The gate intentionally validates the pure coordinate/query/rasterization invariants without linking or executing the game binary, because consumer residency and streaming lifecycle belong to Phase 8 rather than chunk synthesis ownership.

`VoxelChunk` remains a derived runtime representation. Materializing one chunk never changes future generator answers and never requires neighboring chunks to exist.

## Phase 8 — Consumer integration in progress

Current consumer-side implementation:

- `src/world/streaming.rs`
- `src/world/streaming/selection.rs`
- `src/world/streaming/generation.rs`
- `src/world/streaming/presentation.rs`
- `src/world/chunk_visibility.rs`
- `src/hud/chat/locate.rs`
- `tools/check_streaming_materialization_contract.py`
- `tools/check_streaming_presentation_contract.py`
- `tools/check_locate_generator_contract.py`
- `.github/workflows/ci.yml`

Implemented now:

- `ChunkStreamingState` owns only runtime interest/residency policy: current center, movement direction, desired/retired membership, pending materialization requests, in-flight coordinates, and pending presentation membership; it does not own generated biome/terrain/material/Structure truth;
- desired horizontal columns are selected from render distance while their generated vertical span is derived from authoritative `TerrainQueries::sample_surface_area(...)` plus `StructureQueries::placements_intersecting(...)`; a small player-local vertical safety range is included independently of generated semantics;
- streaming does not inspect `VoxelWorld` to infer generated terrain or Structure facts; `VoxelWorld` is consulted only for runtime residency/restoration/publication state;
- newly requested chunks are materialized asynchronously by cloning the immutable `WorldGenerator` and calling `WorldGenerator::materialize_chunk(coord)` in the async compute pool;
- runtime scheduling limits in-flight materializations and per-frame dispatch/integration work without feeding those budgets into generation semantics;
- `TaskInputRevision` rejects completed work from an obsolete generator snapshot; still-desired stale coordinates are requeued instead of publishing stale content;
- a generator resource change restarts the materialization revision and requeues interrupted desired coordinates;
- deterministic request priority is world-coordinate anchored: horizontal distance, total distance, forward/neutral/backward movement band, then explicit Y/Z/X tie-breaks;
- archived chunks take the existing `VoxelWorld::restore_chunk(...)` path before new generation is dispatched; the completion path rechecks restoration so archived runtime state wins even if it becomes available while a generation task is in flight;
- a newly generated `MaterializedChunk` is inserted into `VoxelWorld` before dormant fluid ticks are reactivated or any generated-fluid frontier target is handed to the runtime solver;
- only sparse frontier targets returned by the materializer are offered to `enqueue_generated_fluid_frontier(...)`, and only when the target position is currently resident;
- when a neighboring target chunk becomes resident later, streaming re-evaluates potential generated-fluid frontier sources on already-resident neighboring chunks and enqueues only targets that actually belong to the newly resident chunk; this keeps A→B and B→A load order from changing which boundary targets wake;
- restored chunks reactivate dormant fluid ticks and participate in the same resident boundary-frontier revalidation without being regenerated;
- no filled generated-fluid volume is bulk-enqueued into runtime simulation;
- newly published/restored resident chunks enter the preserved direct-light seed/relaxation path only after `VoxelWorld` residency is established, then enqueue presentation work;
- streaming synchronizes `ChunkPresentationSelection` from its own resident interest state, so presentation cannot outlive/escape the current streaming selection;
- initial presentation reserves render-pool membership without synchronously building meshes and hands all heavy mesh generation to the existing budgeted `ChunkRemeshQueue`;
- first presentation enqueues an all-meshlet remesh plus neighbor halo catch-up, preserving geometry/fluid seam updates when a chunk becomes visible after its neighbors;
- presentation publication is explicitly frame-budgeted and skips retired, missing, or already-presented chunks;
- `/locate biome` validates current authored surface-biome eligibility, clones no generation registries into its worker, calls `BiomeQueries::find_surface_biome(...)`, and resolves display Y through `TerrainQueries::surface_at(...)`;
- `/locate structure` validates locatability/reference authoring, then calls `StructureQueries::find_nearest(...)`; requested group variations use `find_nearest_variant(...)`, which filters the deterministic resolved root inside `StructureField` rather than scanning chunks or rebuilding variation selection in chat code;
- `/locate` workers clone only the immutable `WorldGenerator`, run on the async compute pool, and preserve the existing clickable locate-result behavior without materializing chunks;
- the streaming materialization contract audit freezes generator-owned selection/materialization, stale-result rejection, restored-vs-new publication, resident-before-frontier ordering, sparse frontier handoff, and deterministic priority without linking or executing the game binary;
- the streaming presentation contract audit freezes resident publication before lighting/presentation activation, selection synchronization, remesh-owned initial mesh work, and neighbor halo catch-up;
- the locate generator contract audit rejects `VoxelWorld`, raw seed, chunk materialization/grid traversal, consumer-side Structure area scans, and direct owner-internal dependencies from the chat locate path.

### Phase 8 current boundary

Streaming now owns deterministic materialization/residency policy and reconnects resident chunks into preserved lighting/presentation queues without becoming a generation owner. `/locate biome` and `/locate structure` now answer unmaterialized generated-world questions directly from the immutable generator snapshot. The remaining Phase 8 work is the rest of the generated-world consumer audit/migration, especially spawn/warp/portal destination and grounding queries.

## Next concrete work

Continue **Phase 8 — Consumer integration** with the remaining generated-world query consumers.

Required direction:

1. reconnect spawn/warp and portal destination/grounding queries to authoritative generator terrain/material/Structure capabilities wherever they ask about generated-world facts;
2. audit remaining gameplay/debug consumers for direct generated-world reconstruction or materialized-chunk scans and migrate only the consumers that need generator truth;
3. preserve runtime mutation/residency ownership where the question is about current mutable world state rather than untouched generated-world facts;
4. keep the Phase 8 persistence boundary limited to restored-vs-new chunk identity; the full all-materialized-chunks persistence contract remains Phase 9.

## Important non-regression rules

- No legacy worldgen implementation may be restored for convenience.
- No semantic generation region or chunk-owned biome/terrain/Structure truth.
- No separate land/ocean ownership model.
- No hydrology subsystem; rivers remain generic connected Structures/connectors.
- No duplicate biome map, terrain sampler, material sampler, generated-fluid sampler, Structure planner, or query resolver.
- Queries must remain pure, deterministic, world-coordinate anchored, and independent of generation/query order.
- No compatibility shims for deleted generation contracts unless explicitly requested.
- No `cargo test` in agent workflows/CI unless explicitly requested for the current task.
