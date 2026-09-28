# Asteria Core Rebuild — Implementation Progress

Branch: `architecture/asteria-core-rebuild`  
Plan: `docs/asteria-core-rebuild.md`  
Baseline: `develop@5038934a97a51cddcd8cdc94fc61314cb650d96d`

This file records implementation checkpoints for the long-lived rebuild branch. A checkpoint is only marked complete after the branch validation pipeline passes Clippy with `-D warnings`, `cargo check`, and the existing content audits.

## Phase 0 — ownership inventory

Status: **completed enough to drive the controlled migration**.

Key findings:

- the previous `ChunkStreamingState` owned too many unrelated facts: residency selection, generation request queues, presentation-ready queues, retirement, surface/structure selection caches, initial lighting activation, mesh-pressure state, generation waves, fluid settling and diagnostics;
- the useful generation/mesh async code already had snapshot, cancellation and stale-result concepts worth preserving;
- logical world existence, CPU residency and render residency must remain independent;
- structure intent belongs to world metadata, not disposable calculation caches;
- legacy hydrology remains deleted and is not a migration target.

## Phase 1 — core types and boundaries

Status: **ownership cutover block completed; core-type contract work in progress**.

The streaming resource remains the composition root, but authoritative/runtime facts now have explicit owners. Remaining `ChunkStreamingState` methods are intentional boundaries, cross-owner invariants or diagnostics aggregation; no further wrapper removal is justified solely to reduce method count.

The original roadmap still requires stable world/dimension/region/chunk/voxel identities and narrow framework-light contracts before Phase 1 can be considered complete. The ownership work below is therefore a completed sub-block of Phase 1, not permission to skip directly to performance tuning or authoritative-storage Phase 2.

### Foundation boundaries completed so far

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

### Streaming selection-state cutover

- Added `StreamingSelectionState` as the explicit owner of the current streaming center, movement direction, horizontal radius and vertical radius.
- Rebuild-needed checks, movement-direction updates, warp direction reset and committed selection pose now live behind that owner.
- Selection geometry, forward preload policy, retention radii, priority ordering and incremental rebuild rules were preserved.
- Generation and meshing consume narrow selection accessors instead of reading raw pose/radius fields from the streaming orchestrator.
- Removed staging artifacts used while resolving the cutover; no `.next` or temporary marker files remain in the final tree.
- Main cutover: commit `71a589299b41dfdf5cd2fb79cc43089059a49ed9`; compile/lint follow-ups: `911c8fa969d61c6f8f3d235ecf4d5d7f7b68ffe2`, `deb62eae5a261c55f6df3109ceb880cc30731fb6`.
- Verified checkpoint: `Rust validation` run `36469505105` — success.

### Streaming priority-diagnostics cutover

- Added `StreamingPriorityDiagnostics` as the owner of pending/ready priority-scan timing metrics and drain/reset behavior.
- `PendingChunkQueue` and `ReadyChunkQueue` still return optional scan samples and remain unaware of logging/aggregation.
- `ChunkStreamingState` now only forwards scan samples to the diagnostics owner; it no longer contains atomics, timing accumulation or snapshot construction logic.
- `render_diagnostics` continues consuming the same diagnostic snapshot fields with the same `crate::world` visibility as before the move.
- Added focused coverage proving pending/ready samples accumulate and drain independently.
- Owner introduction: commit `51106617a677f607dbc3cfcadb9f446aa0de7adc`; orchestrator cutover: `c247ed41dccaabed74b3295b0ca429212081bb4e`; visibility follow-up: `4d8198e7b3451a6e651f6d1c4d0b5acef07be487`.
- Verified checkpoint: `Rust validation` run `36470540393` — success.

### Generation-wave delegation cleanup

- `generation.rs` now operates directly on `GenerationWaveState` for lifecycle-local operations such as target reservation, staging, completion, publication, prefetch and cancellation.
- Removed the corresponding pass-through methods from `ChunkStreamingState`.
- Kept `generation_dispatch_work_exists()` because it intentionally combines generation-wave state with the pending request queue.
- Kept `generated_chunk_is_unpublished()` and `generated_fluid_settling_owns_mutation()` as `world`-level boundaries used outside the streaming submodule.
- Kept `mark_ready()` because it enforces a cross-owner invariant across unpublished generation state, mesh-pressure suppression, logical residency and the presentation-ready queue.
- Generation runtime cutover: commit `09aeab959c8b0316234eb826a16b7f8301c38c1d`; orchestrator cleanup: `9f52523deea6c52f33e884181891695df07c7be5`.
- Verified checkpoint: `Rust validation` run `36471295828` — success.

### Typed dimension identity cutover

- Added framework-light `DimensionId` as the stable runtime identity for dimensions; raw authored/save text is converted at the content/persistence boundary instead of remaining the runtime identity type.
- `CurrentDimension` now owns `DimensionId` rather than `String`, and `CurrentDimensionContext::id()` exposes the typed identity.
- Natural spawning, HUD lookup, new-world creation, world activation, save snapshot capture and bootstrap now convert explicitly with `as_str()` only when calling textual content/save APIs.
- No `Deref<str>`, duplicated string field or compatibility shim was introduced to hide boundary conversions.
- Identity introduction: commit `030826eea9892d4ea1e1add63822871e56e8dd7b`; context cutover: `2649d49f5744b59e77eea8309b4e2a07a4c4cc23`; consumer migrations: `8df93109086b6d9213107534ff8bd157a6adcfec`, `f796a83db07310b5ed9dfdb55b058a75e3f45ba9`, `a288e85ee67e3d234945e48b63419b4080e486d6`, `2c69209e6ed4412a5e7ad33edaf8a589415e1539`, `8bd5e5e49d47e2ade9f52ca019cad274943be8f9`, `03e71810de7f3e74c8e42c65454a06890c578927`.
- Focused tests cover valid identity preservation and blank-ID rejection.
- Verified checkpoint: `Rust validation` run `36474736729` — success.

### Typed chunk identity — async task queue boundary

- Added primitive-backed `ChunkCoord` as a distinct chunk identity while keeping Bevy `IVec3` conversion explicit at adapter edges.
- `ChunkTaskQueue` now stores pending async work as `HashMap<ChunkCoord, ...>` rather than using raw `IVec3` as the identity key.
- Existing scheduler APIs still accept/return `IVec3` for this first narrow cutover; queue entry, lookup, cancellation, priority selection and completed-result publication convert explicitly at the queue boundary.
- No scheduling policy, priority ordering, cancellation behavior, task revision semantics or task limits changed.
- Added focused coverage proving negative chunk coordinates survive the typed/Bevy adapter roundtrip.
- Type introduction: commit `b97500563ccd5fc829fb96c17d1a4017d43b8521`; queue cutover: `d14c57b966ec77d6dca073e535804bd7dce3323a`; lint cleanup: `e8bfebd284e0f9822459cc037ae6b0da28aeb07f`.
- Verified checkpoint: `Rust validation` run `36475544543` — success.

## Current ownership shape

```text
ChunkStreamingState (resource-level composition root)
|
+-- StreamingSelectionState
|   +-- center
|   +-- movement direction
|   +-- horizontal radius
|   +-- vertical radius
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
+-- StreamingPriorityDiagnostics
    +-- pending priority-scan metrics
    +-- ready priority-scan metrics
```

### Why the remaining facade methods stay

- residency and mesh-pressure methods used by `chunk_unloading` are intentional boundaries from sibling world systems into streaming state;
- `adopt_structure_top_chunk`, `mark_ready`, `requeue`, `pop_pending_by_priority`, `pop_ready` and renderable-backlog calculations coordinate multiple owners or enforce ordering/invariants;
- initial-presentation methods name the publication/activation protocol, while `forget_initial_lighting_seeded()` is explicitly required by unload/reload lifecycle;
- diagnostics aggregation intentionally reads several owners and therefore belongs at the composition layer.

## Next Phase 1 implementation block — typed core contracts

The next work follows the original Phase 1 roadmap instead of jumping ahead to optimization:

1. **completed:** stable `DimensionId` runtime/domain type and cutover of `CurrentDimension` plus consumers from raw `String` identity;
2. **in progress:** typed world/chunk/voxel coordinate contracts around the existing Euclidean conversion rules, migrating one complete boundary at a time; the async task queue now owns typed chunk identity internally, while scheduler/storage cutovers remain to be evaluated separately;
3. continue replacing semantically distinct raw revision counters at async/storage boundaries with explicit revision types where the distinction prevents invalid publication/mutation;
4. keep authored content deserialization and Bevy resources as adapters around the domain contracts rather than making Bevy resource shape the identity itself;
5. add focused unit/property tests for ID validity and negative-coordinate/chunk-boundary behavior;
6. reassess the remaining Phase 1 exit criteria from `docs/asteria-core-rebuild.md` before entering authoritative-storage Phase 2.

Measured runtime optimization remains mandatory later, but it must not replace or reorder the defined migration phases.

## Rules still in force

- no compatibility scaffolding for obsolete runtime architecture by default;
- no one-ECS-entity-per-voxel representation;
- no legacy hydrology resurrection;
- one authoritative owner per fact;
- logical world, resident world and rendered world are distinct;
- every async result remains revisioned/cancellable;
- ownership boundaries must not be collapsed to make an optimization easier;
- performance changes require measurements before/after, not intuition-only tuning;
- CI must be green before the next block.
