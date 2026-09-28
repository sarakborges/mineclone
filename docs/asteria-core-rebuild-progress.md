# Asteria Core Rebuild — Implementation Progress

Branch: `architecture/asteria-core-rebuild`  
Plan: `docs/asteria-core-rebuild.md`  
Baseline: `develop@5038934a97a51cddcd8cdc94fc61314cb650d96d`

This file records implementation checkpoints for the long-lived rebuild branch. A checkpoint is only marked complete after the branch validation pipeline passes Clippy with `-D warnings`, `cargo check`, and the existing content audits.

## Phase 0 — ownership inventory

Status: **completed enough to drive Phase 1 cutovers**.

Key findings:

- the previous `ChunkStreamingState` owned too many unrelated facts: residency selection, generation request queues, presentation-ready queues, retirement, surface/structure selection caches, initial lighting activation, mesh-pressure state, generation waves, fluid settling and diagnostics;
- the useful generation/mesh async code already had snapshot, cancellation and stale-result concepts worth preserving;
- logical world existence, CPU residency and render residency must remain independent;
- structure intent belongs to world metadata, not disposable calculation caches;
- legacy hydrology remains deleted and is not a migration target.

## Phase 1 — boundaries and ownership cutovers

### Completed

- Introduced typed async task input revisions instead of passing unqualified `u64` revisions through generation/mesh/remesh scheduling.
- Extracted immutable generation-input snapshot ownership from the generation scheduler.
- Extracted mesh/presentation content snapshot ownership from initial-mesh scheduling.
- Narrowed persistence writing so storage consumes persistent chunk coordinates + chunk serialization instead of depending on the complete `VoxelWorld` runtime object.
- Separated `StructureField` ownership from disposable `WorldFeatureFields` caches.
- Removed stale hydrology requirements from the active architecture path; future rivers/lakes/long-form world features remain structure/connector/world-metadata concerns.

### Residency cutover

- Added `ChunkResidencyState` as the authoritative owner of logical `desired` and `retained` chunk sets plus residency selection revision.
- Moved the retired/unload queue into residency ownership.
- Moved retirement negative-scan caching and unload candidate selection into the residency owner.
- Scheduler-facing retirement API now delegates to residency instead of knowing queue representation or cache invalidation rules.
- Verified checkpoint: commit `f359ae54658eaeddbc8be83aa315810f59c5f5a4`, `Rust validation` run `36457391516` — success.

### Pending-generation queue cutover

- Added `PendingChunkQueue`.
- Pending queue membership, critical-player scan miss cache and priority-order cache are now owned together.
- The streaming orchestrator supplies selection/world context and receives the selected coordinate plus optional scan timing for diagnostics; it no longer owns queue cache revisions.
- Ordered iteration required only by tests is compiled only under `cfg(test)`.
- Verified checkpoint: commit `434f06c8b2cfdff7f0617ef0f0a5cba9fe0120e8`, `Rust validation` run `36458250118` — success.

### Ready/presentation queue cutover

- Added `ReadyChunkQueue`.
- Ready queue membership and its negative priority-scan cache now live together.
- The cache key remains revision-aware (`queue revision + residency selection revision + center + render radius`), preserving invalidation behavior without exposing it to `ChunkStreamingState`.
- Existing initial-mesh priority ordering and streaming scan diagnostics are preserved.
- Tests now validate externally observable invalidation behavior instead of reaching into cache internals.
- Verified checkpoint: commit `94deec65b69d9a3f70ea74ef59b39eb7ed93b1a2`, `Rust validation` run `36459086884` — success.

### Generation-wave lifecycle cutover

- Added `GenerationWaveState` as the owner of active generation target reservations, dispatch-pending targets, prefetch reservations, staged generated chunks and the settled-publication queue.
- Generated-fluid settling remains the dynamic simulation implementation; the generation-wave lifecycle now owns the fact that an active generation wave is waiting on that solver instead of leaving unrelated lifecycle fields spread across `ChunkStreamingState`.
- Existing cancellation, stale-result handling, critical-player wave sizing, prefetch promotion and publication budgets were preserved.
- Verified checkpoint: commit `568ee2849e607d05bc8e6bc65655922c35a95aea`, `Rust validation` run `36460032390` — success.

### Initial-presentation cutover

- Added `InitialPresentationState`.
- Initial direct-light seeding membership, retained seed results, first runtime activation and targeted mesh-seed catch-up masks now have one presentation owner.
- Initial meshing consumes this state through narrow scheduler methods instead of manipulating four independent collections.
- Logical residency and generation publication remain independent from initial presentation readiness.
- Verified checkpoint: commit `9518985ba4f8aa03bc10e80351a6acaccde3cca9`, `Rust validation` run `36460842927` — success.

### Mesh-pressure residency cutover

- Added `MeshPressureState` for GPU/presentation meshes evicted only because the mesh-memory high watermark was exceeded.
- Mesh-pressure suppression/recovery no longer stores a raw `HashMap` directly in the streaming orchestrator.
- Logical chunk residency remains unaffected by mesh-memory pressure; recovery only requeues presentation when the chunk is still logically resident.
- Selection pruning and diagnostics now delegate to the mesh-pressure owner.
- Verified checkpoint: commit `75d98f788f04396cbba4a3474421a791f0f2266e`, `Rust validation` run `36461205950` — success.

### Streaming selection-cache cutover

- Added `StreamingSelectionCache` as the explicit owner of transient surface ranges, surrounding support minima and discovered structure-top columns used by residency selection/prioritization.
- These values remain explicitly **derived streaming caches**, not authoritative biome/structure intent.
- Cache pruning moved behind the owner while preserving the previous policy exactly: surface ranges retain their extra two-chunk margin; support minima and structure-top columns retain only the normal selection retention radius.
- Structure-top adoption now delegates cache ownership while preserving the existing behavior that expands desired residency upward when an asynchronously discovered structure exceeds the cached surface column.
- Full/incremental desired-selection algorithms and priority semantics were intentionally left unchanged.
- Main cutover: commit `c0a0a1b2459617001fbfdc50fc1235ca7257054a`; lint-only follow-up: `fe955ad21609c03e8cc0e8c84446ee4998378cf6`.
- Verified checkpoint: `Rust validation` run `36467167446` — success.

## Current ownership shape

```text
ChunkStreamingState (orchestrator, still being reduced)
|
+-- ChunkResidencyState
|   +-- desired
|   +-- retained
|   +-- selection revision
|   +-- retired queue
|   +-- retirement scan cache
|
+-- PendingChunkQueue
|   +-- generation-request membership
|   +-- critical scan cache
|   +-- priority cache
|
+-- ReadyChunkQueue
|   +-- presentation-ready membership
|   +-- renderable scan cache
|
+-- GenerationWaveState
|   +-- target reservations
|   +-- dispatch-pending queue
|   +-- prefetch reservations
|   +-- staged generated chunks
|   +-- settling/publication lifecycle
|
+-- InitialPresentationState
|   +-- initial direct-light seed state/results
|   +-- first runtime activation
|   +-- mesh-seed catch-up masks
|
+-- MeshPressureState
|   +-- pressure-only presentation evictions
|   +-- retained byte accounting
|
+-- StreamingSelectionCache
|   +-- surface ranges
|   +-- surrounding support minima
|   +-- discovered structure-top columns
|
+-- remaining concentration to extract
    +-- selection pose/radii orchestration
    +-- cross-owner diagnostics/orchestration
```

## Next implementation block

Extract the streaming selection pose from `ChunkStreamingState` without changing selection policy:

1. give current center, movement direction, horizontal radius and vertical radius one owner;
2. move rebuild-needed checks and movement-direction transitions behind that owner;
3. preserve the exact behavior where radius-only changes keep the existing movement direction during normal gameplay, while warp disables forward preload/direction;
4. keep forward preload geometry, selection radii and retention math unchanged;
5. retain narrow scheduler accessors for generation/meshing priority until their call sites can be reduced independently;
6. run full CI before touching diagnostics/orchestrator decomposition.

## Rules still in force

- no compatibility scaffolding for obsolete runtime architecture by default;
- no one-ECS-entity-per-voxel representation;
- no legacy hydrology resurrection;
- one authoritative owner per fact;
- logical world, resident world and rendered world are distinct;
- every async result remains revisioned/cancellable;
- each ownership cutover must preserve behavior first, then optimization can be measured separately;
- CI must be green before the next cutover.
