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
+-- remaining legacy concentration to extract
    +-- generation wave reservations/prefetch/publication
    +-- generated-fluid settling ownership
    +-- initial presentation/lighting activation state
    +-- mesh-pressure eviction state
    +-- surface/structure selection caches
    +-- cross-owner diagnostics/orchestration
```

## Next implementation block

Extract generation-wave lifecycle from `ChunkStreamingState` without changing generation behavior:

1. group active generation targets, dispatch-pending targets and prefetch reservations under one owner;
2. move staged generated chunks and settled-publication queue into the same lifecycle owner;
3. retain dynamic generated-fluid settling as a simulation concern, but give the generation-wave owner an explicit boundary for waiting on / consuming settling completion rather than leaving unrelated fields in the streaming orchestrator;
4. preserve cancellation, prefetch promotion and stale-result semantics;
5. keep the existing work budgets and generation priority policy unchanged during this ownership cutover;
6. run full CI before moving to initial presentation/lighting ownership.

## Rules still in force

- no compatibility scaffolding for obsolete runtime architecture by default;
- no one-ECS-entity-per-voxel representation;
- no legacy hydrology resurrection;
- one authoritative owner per fact;
- logical world, resident world and rendered world are distinct;
- every async result remains revisioned/cancellable;
- each ownership cutover must preserve behavior first, then optimization can be measured separately;
- CI must be green before the next cutover.
