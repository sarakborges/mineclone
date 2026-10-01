# Asteria Core Rebuild Plan

Status: **architecture plan / migration roadmap**  
Branch: `architecture/asteria-core-rebuild`  
Baseline: `develop@5038934a97a51cddcd8cdc94fc61314cb650d96d`  
Baseline CI: `Rust validation` run `36212838598` — success

## 1. Decision

Asteria stays on **Rust + Bevy**.

The problem to solve is not the language or the framework choice. The accumulated work has shown that Asteria needs a more specialized voxel/world runtime than the original architecture assumed. The rebuild therefore changes the role of Bevy:

- Rust remains the implementation language.
- Bevy remains the application host, ECS, scheduling framework, asset/input/window/UI/audio integration, and general rendering infrastructure.
- Asteria's world model, generation, streaming, scheduling, simulation and voxel presentation become explicit Asteria-owned subsystems with narrow Bevy adapters.
- The authoritative world must not be represented by whatever happens to be spawned or rendered in Bevy at the moment.
- A custom `wgpu` voxel renderer is **not** part of the initial rebuild. It is an escalation path only if profiling later proves that Bevy render submission / renderer architecture is the remaining bottleneck after the world runtime is corrected.

This is a **controlled reconstruction of the foundation**, not a rewrite of the whole game.

## 2. What is preserved

The rebuild should preserve and migrate existing working domain/content systems whenever they do not depend on the old world-runtime assumptions. Expected reusable areas include:

- item definitions and data-driven behaviors;
- inventory / creative inventory;
- tools and interactions;
- creature definitions, models and textures;
- UI and screen infrastructure;
- localization;
- authored structures and structure groups where their data remains valid;
- block/fluid definitions;
- save-domain concepts that remain semantically valid;
- diagnostics and validation tools that still measure useful boundaries;
- reusable engineering primitives already aligned with `ENGINEERING_PRACTICES.md`.

Algorithms may be reused without preserving their old orchestration. Reusing code is not a reason to keep an invalid ownership boundary.

## 3. What is being rebuilt

The target foundation is divided into explicit layers:

```text
ASTERIA
|
+-- World Core
|   +-- world identity / dimension identity
|   +-- coordinate and region primitives
|   +-- authoritative voxel/chunk storage
|   +-- deterministic world metadata
|   +-- biome-volume field
|   +-- structure intent / connector graph
|   +-- revisions and dirty state
|
+-- World Generation
|   +-- terrain
|   +-- biome volumes
|   +-- structure planning
|   +-- structure materialization
|   +-- decorators
|   +-- deterministic generation jobs
|
+-- Runtime Streaming
|   +-- desired residency calculation
|   +-- priority scheduler
|   +-- bounded queues / backpressure
|   +-- cancellation / stale-result rejection
|   +-- load / generate / retain / evict lifecycle
|   +-- independent work budgets
|
+-- Simulation
|   +-- dynamic fluids
|   +-- lighting
|   +-- block/object updates
|   +-- entity/spawn rules
|   +-- simulation-owned queues and revisions
|
+-- Voxel Presentation
|   +-- render-section lifecycle
|   +-- meshing
|   +-- model/object presentation
|   +-- render caches / pools
|   +-- visibility / future LOD
|
+-- Bevy Application
    +-- ECS gameplay entities
    +-- player / input
    +-- UI / screens
    +-- assets / audio
    +-- camera / window
    +-- adapters into the layers above
```

The dependency direction should trend inward: core domain/storage must not require Bevy types merely because Bevy hosts the application.

## 4. Non-negotiable architecture rules

The rebuild follows `ENGINEERING_PRACTICES.md` and adds these world-runtime rules:

1. **One authoritative owner per fact.** A rendered chunk/entity is never the authoritative world record.
2. **Logical world != resident world != rendered world.** These are separate states with explicit transitions.
3. **Generation is deterministic.** Same seed + definitions + coordinates must produce the same authored result independent of worker completion order.
4. **Async work is revisioned.** Every result that can become stale carries enough identity/revision information to be rejected safely.
5. **Queues and caches are bounded.** Every queue/cache declares owner, ordering, invalidation, lifetime and memory bound.
6. **Expensive work is change-driven.** Do not repeatedly scan/rebuild unchanged world state each frame.
7. **Frame-sensitive work is budgeted.** Generation, integration, meshing, lighting, fluids and unload work do not share one accidental unlimited budget.
8. **Cancellation is normal.** Warp, rapid movement, changed selection or changed world revisions must be able to invalidate irrelevant work without corrupting state.
9. **No hidden cross-layer mutation.** Generation proposes authoritative world data; presentation consumes it. Rendering does not become gameplay state.
10. **No compatibility scaffolding by default.** Old save/runtime behavior is not automatically preserved when it would force the new core to reproduce obsolete architecture. Migrations are explicit decisions.
11. **CI stays green between migration blocks.** A phase does not advance while the branch has errors or warnings in the existing validation pipeline.
12. **Performance changes require measurement.** Establish baseline -> change -> compare. Do not trade architecture for speculative micro-optimizations.

## 5. Hydrology is not coming back

The deleted legacy hydrology architecture is **not** part of this rebuild.

The new core must not recreate `Hydrology`, `BiomeHydrology`, `DimensionHydrology`, a separate river graph, or any equivalent global hydrology subsystem under a new name.

Future world features such as rivers, lakes, cave entrances and other long-form authored terrain features belong to the generalized structure system:

- world metadata determines that a feature/structure is intended to exist;
- connectors describe required continuation points;
- connectors can target a structure or a structure group;
- structure groups provide authored variation;
- the planner can reserve/route feature intent before affected chunks are rendered;
- materialization edits authoritative world data when the relevant region is generated/resident;
- the feature does not need a dedicated parallel world-generation architecture.

**Dynamic fluid simulation is a separate concern and remains valid.** A river/lake structure may place fluid as authored world content; the runtime fluid solver owns later dynamic propagation caused by topology changes. World-feature planning and fluid simulation must remain distinct.

Any remaining documentation that describes the removed legacy hydrology system is historical/stale and must not be treated as a design requirement for the new core.

## 6. Target world lifecycle

A region/chunk should progress through explicit states rather than acquiring behavior implicitly from spawned entities:

```text
Known metadata
    -> requested
    -> data available / generated
    -> authoritative resident
    -> simulation readiness
    -> presentation requested
    -> meshed / prepared
    -> visible
    -> hidden but retained
    -> evicted presentation
    -> evicted residency
```

Not every use case needs every state as a public enum, but the ownership distinction must exist.

Examples:

- A structure can be known in metadata without its blocks existing in memory.
- A chunk can contain authoritative voxel data without a Bevy render entity.
- A render section can be hidden/evicted without deleting the authoritative chunk.
- A chunk can be resident while lighting/mesh results are still pending.
- An async mesh generated for revision 41 must not overwrite revision 43.

## 7. Migration strategy

The old and new cores should coexist only behind explicit migration boundaries. Avoid a long-lived state where every feature writes to both systems.

Each phase must have:

- a narrow scope;
- a documented owner/contract;
- tests or validation for its invariants;
- diagnostics/benchmarks where performance matters;
- an explicit cutover criterion;
- removal of the superseded path immediately after cutover when practical;
- green CI before proceeding.

## 8. Defined implementation phases

### Phase 0 — Baseline and architectural freeze

Goal: capture the current behavior/performance before changing foundations.

Tasks:

- keep `develop@5038934` as the rebuild baseline reference;
- record representative startup/loading, stationary gameplay, movement/streaming and warp diagnostics;
- inventory current world-generation, streaming, chunk-storage, meshing, lighting, fluid and structure ownership;
- identify synchronous work on the main frame and current queue/cache growth rules;
- classify existing code as `reuse`, `adapt`, `replace` or `delete`;
- add missing regression tests for behavior that must survive the rebuild.

Exit criteria:

- baseline logs/metrics are reproducible;
- the migration ownership map is documented;
- no implementation phase begins with unknown authoritative owners.

### Phase 1 — Core types and boundaries

Goal: create framework-light world primitives and contracts.

Tasks:

- define stable world/dimension/region/chunk/voxel identifiers and coordinate conversions;
- define authoritative chunk-content API without exposing storage internals;
- define revisions/generations used across async boundaries;
- separate authored definitions, deterministic metadata, mutable runtime world data and derived presentation data;
- create narrow adapters from Bevy systems into these APIs.

Exit criteria:

- core unit tests run without requiring a Bevy App where practical;
- no new world-domain API accepts broad Bevy resources when a narrow domain type is sufficient.

### Phase 2 — Authoritative chunk/world storage

Goal: make world data independent of rendering and streaming entities.

Tasks:

- implement the authoritative resident chunk store;
- define content revision, simulation revision and presentation dirty signals explicitly;
- centralize mutation so block/fluid/object edits update the correct revisions/queues once;
- make lookup semantics explicit for resident, absent and known-but-not-resident data;
- bound storage/caches and define eviction ownership.

Exit criteria:

- chunks can exist, mutate and be tested without spawning render entities;
- all mutation paths go through one authoritative boundary;
- presentation can be destroyed and recreated without changing world truth.

### Phase 3 — Deterministic world metadata

Goal: let the world know what belongs where before rendering/materialization.

Tasks:

- define biome-volume metadata independent of visible chunks;
- define structure intent/reservation metadata;
- define connector and structure-group planning contracts;
- make metadata queries deterministic and cheap enough for streaming/spawn decisions;
- cache only where validity and memory bounds are explicit.

Exit criteria:

- biome/structure intent can be queried without materializing render chunks;
- spawn and generation code can ask the metadata layer rather than inferring from nearby visible terrain.

### Phase 4 — Streaming scheduler v2

Goal: replace frame-coupled streaming with explicit desired-state scheduling.

Tasks:

- compute desired residency/presentation separately;
- use stable priority ordering and deterministic tie-breakers;
- introduce bounded work queues with backpressure;
- separate generation, integration, simulation-readiness, meshing and eviction budgets;
- attach request generation/revision IDs and reject stale results;
- support cancellation/abandonment after warp or rapid movement;
- avoid rescanning unchanged queues/regions each frame.

Exit criteria:

- moving/warping cannot cause obsolete work to publish over current state;
- queue sizes are observable and bounded;
- gameplay remains responsive while background work progresses incrementally.

### Phase 5 — Terrain generation v2

Goal: make base terrain generation a deterministic job producing world data, not render side effects.

Tasks:

- isolate terrain calculation from Bevy/rendering;
- make job inputs immutable snapshots/definitions + seed + coordinates;
- publish results atomically only if still relevant;
- remove synchronous generation dependencies from the frame path;
- benchmark generation independently.

Exit criteria:

- terrain jobs are deterministic and independently benchmarkable;
- generating terrain does not spawn meshes/entities or mutate UI/render state.

### Phase 6 — Structures, connectors and feature planning

Goal: make generalized structures the only authored long-form world-feature mechanism.

Tasks:

- migrate structure placement to the new authoritative store;
- make connectors reference specific structures or structure groups;
- support deterministic variation selection;
- define reservation/conflict semantics for structures crossing chunk boundaries;
- support caves/tunnels and future river/lake-like features through the same mechanism;
- remove any surviving parallel/legacy feature-generation paths.

Exit criteria:

- structure intent can cross unloaded regions without forcing them to render;
- structure materialization is deterministic and chunk-boundary safe;
- no legacy hydrology subsystem exists or is required.

### Phase 7 — Voxel presentation / meshing v2

Goal: make rendering a disposable projection of authoritative world state.

Tasks:

- define render-section identity and source revision;
- capture immutable snapshots/halos for background meshing;
- reject stale mesh results;
- pool/reuse GPU-facing assets where measurements justify it;
- make visibility/retention independent of authoritative chunk existence;
- minimize per-frame entity spawn/despawn churn;
- preserve immediate feedback for interactive edits without making the main thread rebuild unrelated sections.

Exit criteria:

- destroying all render sections and rebuilding them produces the same visible world;
- no render entity owns authoritative voxel state;
- meshing cost and submission cost are separately observable.

### Phase 8 — Lighting and dynamic simulation integration

Goal: attach simulation to authoritative revisions without coupling it to world generation or presentation.

Tasks:

- migrate lighting readiness/dirty propagation to the new chunk lifecycle;
- keep lighting work change-driven and budgeted;
- migrate dynamic-fluid scheduler to the new mutation/storage boundary;
- keep authored placed fluid separate from runtime propagation;
- ensure simulation publishes presentation dirtiness through revisions/queues rather than direct rendering mutations.

Exit criteria:

- simulation can run with rendering disabled;
- loading/generated content is not redundantly simulated merely because it became resident;
- lighting/fluid backlog cannot grow unbounded without diagnostics/backpressure.

### Phase 9 — Entities, objects and biome-aware spawning

Goal: make entity placement query world truth/metadata instead of current render accidents.

Tasks:

- use biome-volume and authoritative voxel queries for spawn eligibility;
- define ownership of static world objects separately from their visual entity representation;
- ensure block destruction converts supported placed objects to drops according to gameplay rules instead of deleting presentation-only entities;
- keep creature/entity spatial queries independent from chunk render lifecycle where possible.

Exit criteria:

- a creature cannot spawn in a biome merely because an overlapping visible chunk reports the wrong surface context;
- unloading presentation does not silently delete authoritative placed-world state.

### Phase 10 — Persistence and save cutover

Goal: serialize the new authoritative model, not transient runtime machinery.

Tasks:

- define save schema around authoritative world facts and necessary pending simulation state;
- exclude disposable render caches, tasks and presentation state;
- decide explicitly whether old save migration is worth supporting;
- validate roundtrip determinism/invariants;
- version the save format if semantics changed.

Exit criteria:

- save/load roundtrip reconstructs the same authoritative state;
- no transient async/render identity is required for persistence correctness.

### Phase 11 — Full cutover and legacy deletion

Goal: finish the migration instead of maintaining two engines.

Tasks:

- switch all gameplay consumers to new world APIs;
- delete superseded storage/streaming/generation/presentation paths;
- remove compatibility shims that no longer serve a live migration;
- update `ARCHITECTURE.md` to describe only the final architecture;
- update diagnostics to the final ownership boundaries;
- perform full regression/CI/content audits.

Exit criteria:

- one authoritative world path remains;
- no feature writes to both old and new cores;
- no dead legacy subsystem remains "just in case";
- documentation matches implementation.

### Phase 12 — Renderer decision gate

Goal: decide from evidence whether Bevy rendering remains sufficient.

Only after the preceding architecture is stable:

- profile frame time vs Main schedule vs Render schedule vs GPU/presentation;
- measure draw submission, mesh upload churn, material/asset counts and visibility work;
- if the renderer is demonstrably the dominant remaining bottleneck, prototype a specialized voxel render path with `wgpu` behind the existing presentation boundary;
- otherwise keep Bevy rendering and avoid unnecessary engine work.

This phase is a **decision gate**, not a promised rewrite.

## 9. Performance acceptance principles

The rebuild is successful when performance comes from doing less unnecessary work, not merely from larger hardware budgets.

Required properties:

- no unbounded per-frame world scan;
- no generation/meshing/simulation operation may monopolize the frame because its queue is large;
- unchanged world regions should approach zero CPU orchestration cost outside necessary visibility/simulation work;
- warps and fast movement discard irrelevant work instead of completing it for historical positions;
- hidden/unloaded presentation does not imply lost world metadata;
- main-thread work remains measurable by subsystem;
- memory scales with declared residency/retention policies rather than historical travel distance;
- diagnostics themselves remain bounded and observational.

Numerical budgets/targets should be set from Phase 0 measurements and updated only with recorded evidence.

## 10. Testing strategy

The new base should favor small invariant tests plus a few real integration scenarios.

Minimum coverage by the end of the migration:

- coordinate and chunk-boundary property tests;
- deterministic generation for fixed seed/definitions;
- scheduler priority, fairness, cancellation and stale-result tests;
- queue/cache bound and invalidation tests;
- structure connector determinism and cross-chunk placement tests;
- save roundtrip tests;
- render-result stale revision rejection;
- lighting/fluid mutation integration tests;
- biome-volume spawn eligibility tests;
- warp / rapid movement integration scenario;
- repeated load/unload/return scenario proving no historical memory growth beyond policy.

## 11. Work discipline during the rebuild

For every implementation block:

1. Read the current handoff and this plan.
2. Confirm the previous block's CI is green before starting the next block.
3. State the invariant/owner being changed.
4. Add or update tests/diagnostics first when they are needed to prove the change.
5. Implement one migration boundary at a time.
6. Compare behavior/performance against the recorded baseline where relevant.
7. Remove superseded code immediately after successful cutover when practical.
8. Update `HANDOFF.md` after each committed block with commit, CI and next exact step.

Do not mix unrelated gameplay/content work into this branch unless required to complete a migration boundary.

## 12. First execution sequence

The first coding work after this planning commit should be:

1. Phase 0 ownership inventory of the current world pipeline.
2. Record fresh baseline performance logs using the existing `frame_*`, `main_work_*` and `render work` diagnostics.
3. Produce a concrete `reuse / adapt / replace / delete` map for current modules under `src/world` and related generation/rendering modules.
4. Design the Phase 1 core types and dependency boundary from that inventory.
5. Only then begin moving runtime behavior.

No implementation should start by deleting the entire existing world stack at once. The current implementation remains the behavioral reference until each replacement slice has a testable cutover.

## 13. Final target

The intended end state is not "Bevy but with more optimization patches". It is:

> **Asteria is a specialized voxel/world engine implemented in Rust, hosted by Bevy, with its own authoritative world core, deterministic metadata/generation, bounded streaming scheduler, simulation boundaries and disposable voxel presentation layer.**

That separation keeps the option to replace individual infrastructure pieces later — including the voxel renderer — without rewriting gameplay, content or the authoritative world model again.
