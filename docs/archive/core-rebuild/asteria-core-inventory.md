# Asteria Core Rebuild — Phase 0 Ownership Inventory

Status: Phase 0 working inventory  
Branch: `architecture/asteria-core-rebuild`  
Baseline: `develop@5038934a97a51cddcd8cdc94fc61314cb650d96d`

This inventory is the migration map for the controlled Asteria core rebuild. It is intentionally about **ownership and contracts**, not file shuffling. A file marked `adapt` can keep most of its implementation while losing ownership it should not have. A file marked `replace` can still donate algorithms and tests; the old orchestration/lifecycle must not survive merely because code is reusable.

Classification meanings:

- **reuse** — current responsibility and ownership are already aligned with the target architecture; preserve with only incidental changes.
- **adapt** — useful implementation and/or data model, but boundary, ownership, API, or dependency direction must change.
- **replace** — current owner/orchestrator encodes the wrong lifecycle or combines responsibilities that must become separate owners.
- **delete** — legacy concept must not exist in the target core.

## 1. Current authoritative state and ownership findings

### `voxel::world::VoxelWorld` — adapt, then split

Current ownership observed:

- resident voxel chunks;
- loaded vertical-column index;
- archived persistent chunks;
- persistent-chunk membership;
- chunk content revisions;
- chunk mesh revisions;
- chunk object revisions;
- global object-scene revision;
- global block-content revision;
- mutation APIs for blocks, fluids, layers and objects;
- archive/restore lifecycle.

Good parts to preserve:

- one mutation surface for voxel facts;
- explicit revision bumps;
- compact archived persistent chunks;
- deterministic coordinate helpers and resident-column index;
- generated state becoming persistent only after authoritative mutation.

Target change:

- **World Store** owns canonical chunk content and persistent-vs-derived state.
- **Residency** owns whether canonical/derived chunk content is resident, archived, absent, loading or being evicted.
- **Presentation revisions** do not belong in the canonical store. Mesh dirtiness/publication state moves to Voxel Presentation.
- **Object-scene presentation revisions** move to the owning object presentation/index layer unless they represent canonical object content.
- mutation of canonical voxel content keeps a content revision; consumers derive their own invalidation from it.

Do not rewrite `VoxelChunk` just to achieve the split. First put ownership boundaries around the current representation.

### `world::streaming::ChunkStreamingState` — replace

This is the strongest Phase 0 ownership violation. One resource currently owns or caches all of the following:

- streaming center and movement direction;
- desired and retained sets;
- retired/pending/ready queues;
- surface ranges and support minimums;
- structure top-chunk cache;
- initial-lighting seeding/readiness state;
- initial mesh catch-up;
- mesh-pressure eviction bookkeeping;
- generated-fluid settling;
- generation wave targets and queues;
- generation prefetch targets;
- staged generated chunks;
- settled publication chunks;
- selection revision;
- scan-miss and priority caches;
- streaming diagnostics.

Target owners:

- `ResidencyPlan` — desired/retained/retire intent + plan revision;
- `ChunkLifecycleTable` — per-chunk lifecycle state;
- `GenerationScheduler` — generation request queue/in-flight work;
- `PresentationScheduler` — mesh/presentation readiness and work;
- lighting owner — lighting readiness/settling;
- fluid simulation owner — dynamic fluid scheduling only;
- diagnostics — observational counters only;
- selection/index caches — owned by the algorithm whose inputs they accelerate.

The existing priority math, stable tie-breaks, scan-miss caches, render-distance policy and queue primitives are candidates for reuse. `ChunkStreamingState` itself is not.

### `world::setup::WorldLoadingState` — replace as orchestrator

It currently serializes world creation into a single phase machine (`Generating -> SettlingFluids -> Lighting -> Meshing -> Assets -> Finalizing -> Spawning`) and owns many counters/cursors plus structure-column extension and generated-fluid settling.

Target:

- loading screen observes the same runtime lifecycle used in gameplay;
- no separate bootstrap-only chunk ownership model;
- initial spawn residency is a high-priority `ResidencyPlan`, not a second world pipeline;
- the UI can still expose human-friendly stages, but stages are read models over independent owners, not ownership of the work itself.

The progress UI and status types can be adapted. The giant bootstrap execution state is replaced.

## 2. Module migration map

| Current area | Decision | Target / notes |
| --- | --- | --- |
| `voxel/chunk.rs` and voxel cell storage | reuse | Core chunk content representation; keep framework-independent behavior where practical. |
| `voxel/coordinates.rs` | adapt | Keep Euclidean coordinate algorithms; introduce strong core coordinate types and Bevy/glam adapters at boundaries. |
| `voxel/world.rs` | adapt/replace ownership | Preserve mutation semantics and chunk data while splitting canonical store, residency/archive state and presentation dirtiness. |
| `voxel/chunk_archive.rs` | reuse/adapt | Compact persistent archive is useful; move behind storage/persistence boundary. |
| `voxel/chunk_disk.rs` / disk encoding | reuse | Persistence encoding remains a persistence concern, never runtime residency state. |
| `world/biome_field*` | reuse/adapt | Deterministic biome-volume algorithms are core world metadata. Remove framework-resource assumptions from inner calculation over time. |
| `world/density_sampling*` | reuse | Pure generation algorithm; feed from explicit generation snapshot. |
| `world/terrain.rs`, noise/math helpers | reuse | Deterministic pure-ish generation primitives. |
| `world/generation.rs` passes | adapt | Keep terrain/material/fluid/structure algorithms; generation becomes a pure request -> result boundary with immutable input snapshots. |
| `world/generation_region.rs` | reuse | Spatial generation primitive if it remains deterministic and framework-light. |
| `world/structure_field.rs` | adapt | Strong candidate for the future structure intent/index owner. Stop treating it as a hidden cache subsidiary of streaming. |
| `world/world_feature_fields.rs` | replace as umbrella | Split independent caches/indexes by owner. `StructureField` survives; miscellaneous `FeatureCaches` must declare per-cache validity and memory policy. |
| `world/world_feature_fields/cache.rs` | adapt/split | Reuse proven caches only under explicit owner, key, invalidation and bound. |
| `world/chunk_task_queue.rs` | reuse/adapt | Generic revisioned async task registry is valuable. Generalize identity/revision without coupling correctness to streaming. |
| `world/chunk_async_work.rs` | adapt | Keep bounded permits/backpressure; scheduling policy moves to the relevant scheduler. |
| `world/chunk_generation_tasks.rs` | adapt | Immutable generation snapshots + revision rejection are good; ownership moves to `GenerationScheduler`. |
| `world/chunk_mesh_tasks.rs` | adapt | Immutable mesh snapshots + dependency capture are good; ownership moves to Voxel Presentation. |
| `world/chunk_remesh_tasks.rs` | adapt | Keep dependency/revision validation; presentation owns it, not world residency. |
| `world/chunk_remesh/queue.rs` | reuse/adapt | Keep dedupe/coalescing/fairness mechanics. Input becomes presentation invalidation intent. |
| `world/chunk_rendering*` | adapt | Explicit Voxel Presentation layer. Render pool is derived state, never world truth. |
| `world/chunk_visibility.rs` | adapt | Visibility consumes residency/presentation state; must not control residency correctness. |
| `world/chunk_unloading.rs` | replace orchestration | Split canonical eviction/archive from mesh residency/pressure eviction. These are different lifecycles. |
| `world/streaming/selection.rs` | reuse/adapt | Keep deterministic selection/priority algorithms behind `ResidencyPlanner`. |
| `world/streaming/surface_cache.rs` | adapt | Move to the metadata/selection owner; explicit invalidation and bound. |
| `world/streaming/generation.rs` | replace orchestration | Currently mixes generation integration, generated-fluid settling, lighting/remesh publication and residency. Split by owners. |
| `world/streaming/meshing.rs` | replace orchestration | Presentation scheduler consumes resident-ready canonical chunks. |
| `world/streaming.rs` | replace | New thin coordination adapter only after child owners exist. |
| `world/lighting_updates.rs` + voxel lighting | adapt | Keep solver and revision ideas; lighting gets its own lifecycle/readiness owner and budget. |
| `world/fluid_updates*` | adapt | Keep scheduled-tick dynamic fluid simulation. Remove world-feature planning concepts and bootstrap ownership from it. |
| `world/setup*` | replace orchestration | Loading becomes a client/read model of the same schedulers used by gameplay. |
| `world/save*`, `save_catalog*`, `save_session*` | adapt | Preserve generation publication/durable semantics; depend on canonical persistent state through an explicit snapshot boundary. |
| `world/chunk_storage.rs` | reuse/adapt | Region/generation atomic publication is valuable; stop accepting the full runtime `VoxelWorld` as the persistence abstraction long-term. |
| `world/render_diagnostics*` | reuse | Observational only. Preserve while rebuilding because it is needed for before/after evidence. |
| `world/work_budget.rs` | reuse | Keep common bounded frame-work primitive. New schedulers receive independent sub-budgets/policies. |
| `world/tick.rs` | reuse | Simulation time owner remains independent of wall-clock/render speed. |
| `world/warp.rs` | adapt | Warp changes desired residency and cancels stale work; it must not directly invent alternate lifecycle semantics. |
| UI, inventory, content registries, creatures, tools | reuse/adapt only as needed | Outside rebuild scope unless they currently depend on a world ownership leak. |

## 3. Delete list

The target architecture must not contain any of these concepts as authoritative subsystems:

- legacy Hydrology / `BiomeHydrology` / `DimensionHydrology`;
- a dedicated river/lake worldgen graph parallel to the structure system;
- bootstrap-only world ownership separate from gameplay residency;
- render entities or render-pool membership as proof that a chunk exists;
- one mega streaming state that owns generation, lighting, fluids and presentation readiness;
- unrevisioned async publication;
- unbounded queues/caches whose growth follows explored distance forever;
- duplicate canonical chunk state in both runtime ECS and the voxel store.

Future rivers, lakes, cave entrances and similar authored long-form features use structure intent + connectors + structure groups + world metadata. Dynamic fluid simulation remains separate.

## 4. Target ownership matrix

### World Core / metadata

Owns:

- world and dimension identity needed by deterministic generation;
- strong spatial identities;
- canonical chunk content interface;
- persistent-vs-derived mutation ownership;
- biome-volume metadata;
- structure intent/index metadata;
- canonical content revisions.

Does **not** own:

- Bevy entities;
- mesh handles;
- cameras;
- visibility;
- loading-screen progress;
- GPU resources.

### Residency

Owns:

- desired set;
- retained set/policy;
- per-chunk lifecycle state;
- archive/restore/eviction requests;
- residency plan revision;
- cancellation of residency work that became irrelevant.

Does not own generation algorithm, lighting algorithm, fluid simulation or rendering.

### Generation scheduler

Owns:

- generation requests;
- bounded in-flight tasks;
- immutable generation input snapshot identity;
- result revision validation;
- cancellation/requeue policy.

Publishes generated chunk content only through the canonical world-store boundary.

### Simulation

Lighting owns lighting state/readiness/work. Dynamic fluid simulation owns scheduled fluid ticks. Neither subsystem decides whether a world feature such as a river should exist.

### Voxel Presentation

Owns:

- mesh tasks;
- mesh dependency revisions;
- render pool/handles;
- visibility;
- presentation memory pressure and LOD policy;
- presentation readiness.

A presentation object may disappear while canonical/resident world state remains valid.

### Persistence

Owns durable serialization/publication. It consumes an explicit canonical persistent snapshot/catalog and never reaches into render or streaming implementation details.

## 5. Lifecycle contract to implement

The target chunk lifecycle is not a single enum shared by every subsystem. It is the composition of independent facts with one owner each:

```text
World existence/content:
  absent-derived | generated-resident | persistent-resident | persistent-archived

Residency intent:
  unwanted | retained | desired

Generation work:
  none | queued(revision) | running(revision)

Lighting:
  unavailable | dirty | settling | ready(content_revision)

Presentation:
  absent | queued(dependencies) | building(dependencies) | resident(dependencies)
```

Invalid combinations are prevented at boundaries. Examples:

- presentation cannot publish if canonical dependency revisions changed;
- lighting cannot publish ready for a chunk that is no longer resident;
- a generation result cannot overwrite resident or persisted canonical content;
- a stale residency-plan request cannot resurrect a chunk after a warp;
- presentation eviction never archives/deletes canonical chunk content.

## 6. Phase 1 implementation order derived from inventory

1. Introduce framework-light core identities/revision types without changing runtime behavior.
2. Introduce explicit canonical chunk-store capability around current `VoxelWorld` behavior.
3. Introduce per-chunk residency intent/lifecycle model alongside current streaming code.
4. Move desired/retained selection out of `ChunkStreamingState` behind a `ResidencyPlanner` boundary.
5. Move generation task ownership behind `GenerationScheduler`, initially adapting existing task queue/snapshot code.
6. Integrate results through the canonical store boundary with plan/input revisions.
7. Only after parity, remove corresponding fields and methods from `ChunkStreamingState`.
8. Repeat for lighting readiness, generated-fluid bootstrap, meshing/presentation and eviction.

Each numbered cutover must compile cleanly, pass Clippy with `-D warnings`, and preserve deterministic tests before the superseded path is deleted.

## 7. Immediate contradictions / debt discovered in Phase 0

### Stale hydrology rules in `ARCHITECTURE.md`

The current canonical architecture text still contains historical `BiomeHydrology`, `DimensionHydrology`, river/lake and hydrology-specific cave/ocean rules even though the code direction removed that subsystem. These paragraphs are **legacy documentation** and are not authority for the rebuild. They must be removed/reworded before the branch is considered architecture-complete. `docs/asteria-core-rebuild.md` and this inventory are authoritative for the rebuild on this branch: hydrology is deleted and must not return.

### Generated-fluid settling is coupled to streaming generation waves

`streaming/generation.rs` currently pauses generation publication around `GeneratedFluidSettling`, then emits lighting/remesh/frontier work as part of the same generation-wave orchestration. This is a migration hotspot. The target keeps any necessary initial fluid-state convergence as a generation/simulation contract, but the streaming owner must stop owning fluid simulation.

### Presentation and canonical eviction are coupled

Current unload/mesh-pressure behavior lives near the same streaming lifecycle. The rebuild must distinguish:

- evicting GPU/mesh presentation;
- dropping deterministic unmodified resident chunk content;
- archiving authoritative persistent chunk content.

They are three different operations with different owners.

## 8. Phase 0 completion criterion

Phase 0 is complete when:

- this inventory is committed;
- every world-runtime area has a migration decision;
- the deleted-hydrology contradiction is explicitly recorded;
- the first Phase 1 core types/boundaries are implemented with tests;
- no existing behavior has been removed before its replacement boundary exists.
