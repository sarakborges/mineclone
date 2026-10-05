# World Systems Rebuild — Implementation Status

This document tracks the current implementation state of the world-systems rebuild. The architecture and behavioral contract remain in [`world-systems-rebuild.md`](world-systems-rebuild.md); this file records only what is implemented now, what remains, and the next concrete cut.

Implementation branch: `world-systems-rebuild`

Code baseline recorded here: `f5a420572605ef7dd680a65cf96a57314b6bcb92` (`Simplify loading screen query types`). Rust validation run `37350697360` completed successfully for this baseline, including all content/localization/GLB audits, world-generation ownership contract gates, Phase 9 persistence, Phase 10 loading-pipeline, Phase 11 loading-screen audits, Clippy with `-D warnings`, and `cargo check --locked`.

## Validation policy

Repository validation follows root `AGENTS.md`.

- Do not run or add `cargo test` unless the user explicitly requests that command for the current task.
- Default Rust validation is Clippy with `-D warnings`, `cargo check --locked`, and applicable lightweight contract/content audits.
- Visual/debug validation remains an on-demand capability rather than a permanent full-game CI probe.
- A milestone is not complete while the exact delivered SHA has pending, failed, or unobservable CI.

Current lightweight CI gates:

- `tools/check_structure_query_contract.py` — Structure bounded-query, ordering, nearest, and authored probe invariants.
- `tools/check_chunk_materializer_contract.py` — dense semantic sampling and block → fluid → Structure → frontier composition order.
- `tools/check_chunk_seams.py` — adjacent-chunk partition/seam, Structure clipping, attachment projection, and fluid-frontier invariants.
- `tools/check_streaming_materialization_contract.py` — generator-owned selection/materialization, revision rejection, restore-vs-new publication, and deterministic priority.
- `tools/check_streaming_presentation_contract.py` — resident-before-presentation, direct-light activation, remesh ownership, and neighbor halo catch-up.
- `tools/check_locate_generator_contract.py` — `/locate` consumes generator query capabilities and never scans runtime chunks to reconstruct generated facts.
- `tools/check_destination_query_contract.py` — spawn/warp destination selection uses the shared generator-backed primitive and minimal runtime validation.
- `tools/check_materialized_chunk_persistence_contract.py` — every materialized chunk, including unedited/empty chunks, becomes persisted spatial state; save format is v8.
- `tools/check_loading_pipeline_contract.py` — new/load/dimension entry share the replacement Loading pipeline, real required-residency progress, stale-result rejection, and render bootstrap.
- `tools/check_loading_screen_contract.py` — Loading UI renders only authoritative phase/counters and rejects fake timer/duration progress.

## Phase status

| Phase | Status | Current state |
| --- | --- | --- |
| 0 — Impact audit | Complete | External consumers and required capability boundaries are documented. |
| 1 — Cleanup | Complete | Legacy biome/world-generation/loading ownership and transitional generation bridges were removed; preserved runtime systems remain separate. |
| 2 — Generation foundation | Complete | Immutable generation snapshots, deterministic named entropy, world-coordinate queries, and scalar/bounded sampling primitives are implemented. |
| 3 — Biome Layout | Complete for current authored content | Authoritative surface layout, influences, search, `regionSize`, `cannotBorder`, and biome-map rendering are implemented. No volume-biome content is currently authored. |
| 4 — Terrain | Complete for current authored content | Authoritative continuous base surface, final 3D density, caves, floating masses, bounded effective surfaces, scalar/batch queries, and terrain debug validation are implemented. |
| 5 — Surface/materials/generated fluids | Complete for current authored content | Solid layers, deterministic patches, Ocean water, swamp puddles, volcano lava pools, scalar/batch fluid queries, and generated-fluid runtime-frontier handoff are implemented. |
| 6 — Structures/features | Complete for current authored content | `StructureField` owns deterministic placement/query, arbitration, StructureSets, connectors, current surface roots, biome-margin roots, lake/river and waterfall/pond chains. |
| 7 — Chunk synthesis | Complete for current authored content | `ChunkMaterializer` composes dense solid/material results, generated fluids, planned Structures, attachments, and sparse generated-fluid frontier targets into one `VoxelChunk`. |
| 8 — Consumer integration | Complete for current active consumers | Streaming, `/locate`, spawn, warp and dimension travel consume generator/query capabilities. Portal/Dimensional Slicer has no active consumer yet; its future implementation must use the same destination-loading contract. |
| 9 — Persistence | Complete | All materialized chunks are authoritative persisted per-dimension spatial state. Unedited and empty chunks are included; query-only access does not persist. Save format v8 intentionally breaks the old mutation-only semantic contract. |
| 10 — Loading pipeline | Complete | New world, save load, and dimension travel share one Loading path. The same streaming owner materializes the required destination neighborhood; progress is real residency work; render resources are bootstrapped without restoring the legacy loader. |
| 11 — Loading screen | Complete | Replacement UI renders `WorldLoadingProgress` directly with authoritative phase and completed/total residency counters. No timer, duration, or parallel loading state exists. |
| 12 — End-to-end/performance | In progress | Correct replacement stack now has a validated end-to-end entry path. Next work is correctness/fixture coverage and the first measured performance baseline. |

## Authoritative ownership summary

### Generation

`src/world/generator/` remains the immutable generated-world owner. Biome, terrain, material, generated-fluid and Structure facts are deterministic functions of seed, dimension definitions and world coordinates. Runtime chunk state is derived materialization, never a second generation owner.

Key invariants:

- no semantic generation region or chunk-owned biome/terrain/Structure truth;
- scalar and bounded/batch queries share semantics;
- queries do not require chunk materialization;
- no query/generation-order dependence;
- no duplicate biome map, terrain sampler, fluid sampler or Structure planner;
- rivers/lakes/waterfalls remain generic Structure/connector content rather than a parallel hydrology owner.

### Runtime materialization and streaming

`ChunkMaterializer` is the semantic-to-runtime adapter. `ChunkStreamingState` owns only runtime interest/residency policy and async materialization scheduling.

Current behavior:

- generated vertical selection comes from authoritative terrain/Structure queries;
- newly requested chunks call `WorldGenerator::materialize_chunk(coord)` asynchronously;
- stale async results are rejected by generator revision;
- archived persisted chunks restore before new generation can replace them;
- generated chunks become resident before lighting/frontier/presentation activation;
- only sparse generated-fluid frontier targets are handed to the runtime solver;
- presentation/remeshing stays a budgeted runtime continuation and is not a generation owner.

### Generated-world consumers

Existing generated-world consumers use narrow capabilities rather than reconstructing worldgen:

- `/locate biome` uses biome search + terrain surface queries;
- `/locate structure` uses authoritative Structure search;
- spawn uses the shared safe-destination query primitive;
- warp uses direct requested-destination preparation plus minimal current-world collision/support/fluid validation;
- dimension travel switches dimension runtime state then re-enters the same Loading pipeline;
- future portals/Dimensional Slicer must reuse this direct destination-loading contract instead of introducing another loader or search path.

## Phase 9 — Persistence complete

The old semantic contract — persist only mutated chunks and regenerate untouched terrain from seed — is gone.

Current persistence contract:

- a chunk becomes authoritative persisted spatial state when it is first materialized into the playable world;
- resident unedited chunks are included in save enumeration;
- unloaded materialized chunks are archived rather than discarded;
- empty materialized chunks still persist their coordinate identity;
- restored chunks are never regenerated over;
- active and inactive dimensions pass through the same per-dimension chunk storage path;
- query-only generator access does not materialize or persist chunks;
- save format v8 separates the new semantics from pre-rebuild mutation-only saves.

The existing atomic generation/staging/region publication shell was preserved instead of creating a second storage subsystem.

## Phase 10 — Loading pipeline complete

The replacement Loading pipeline is shared by new-world entry, save loading and dimension travel.

Current sequence:

1. establish/resume world session state;
2. install the immutable `WorldGenerator` for the active dimension;
3. install/reuse world render resources (`TerrainLightingBuffer`, `TerrainMaterials`, `FluidMaterials`);
4. restore or choose the destination through the shared generator-backed destination primitive;
5. spawn/restore the player at that target;
6. move streaming interest directly to a small required destination neighborhood;
7. restore or materialize required chunks through normal streaming/materialization;
8. publish real `completed/total` residency counters;
9. enter Gameplay only after all required chunks are resident and materialization is idle.

Heavy presentation/remeshing remains normal Gameplay continuation instead of being duplicated inside Loading. Texture-array asset preparation advances during Loading/Gameplay without becoming another generation phase.

## Phase 11 — Loading screen complete

`src/screens/loading_screen.rs` is a renderer of `WorldLoadingProgress` only.

The screen displays:

- the existing localized loading title;
- the authoritative logical phase;
- real completed/total required chunk residency;
- a ratio-derived progress bar.

The screen owns no timer, duration, wall-clock estimate, materialization queue, generator state, or parallel progress model. It is scoped to `GameState::Loading` and despawns on state exit.

## Phase 12 — current work

The design requires end-to-end correctness and the first evidence-based performance baseline before the rebuild is mergeable.

Required coverage:

- fixed-seed and multi-seed generation determinism;
- near/far-coordinate query/materialization behavior;
- save/load preservation of materialized spatial state;
- dimension travel and warp;
- `/locate` biome/Structure queries;
- streaming/request-order behavior;
- visual fixtures for biome borders/junctions, `cannotBorder`, Ocean/coast, caves, floating terrain, Structure-heavy areas, Overworld and Umbral;
- performance measurement for biome scalar/area queries, biome-map rendering, terrain scalar/area generation, Structure search, chunk synthesis, initial playable-area preparation, far-coordinate warp, locate, and relevant temporary memory/cache growth;
- cold and warm paths where caching changes cost but not semantics.

No arbitrary millisecond threshold will be invented. The first correct measurements establish the baseline; concrete regression budgets come only after evidence exists.

## Next concrete work

Continue **Phase 12 — end-to-end validation and performance**.

1. Build a lightweight deterministic end-to-end fixture/audit that exercises new/load/dimension/warp/query/materialization boundaries without restoring a second generation path.
2. Add targeted diagnostics/benchmark entry points for generator queries, Structure search, chunk synthesis, initial required-area loading, far-coordinate destination preparation and locate.
3. Record the first fixed-seed near/far and cold/warm baseline rather than guessing thresholds.
4. Audit the live playable path for runtime resource/scheduling gaps that source-shape contracts cannot detect.
5. Keep full visual/debug probes on demand when they require linking/running the game binary; CI remains lightweight unless a reliable bounded probe is demonstrated.

## Non-regression rules

- No legacy worldgen implementation may be restored for convenience.
- No compatibility shim for deleted generation contracts unless explicitly requested.
- No semantic generation region or chunk-owned generated truth.
- No duplicate land/ocean, hydrology, biome-layout, terrain, generated-fluid or Structure owner.
- Queries stay pure, deterministic, world-coordinate anchored and independent of request order.
- Runtime mutable-world questions may inspect `VoxelWorld`; untouched generated-world facts must come from generator capabilities.
- Materialized chunks stay persisted spatial truth; unmaterialized territory stays generator-owned.
- Loading UI renders authoritative progress only; it never manufactures progress.
- No `cargo test` in CI/agent workflows unless explicitly requested for the current task.
