# World Systems Rebuild Plan

Status: **active planning and implementation plan**  
Working branch: `world-systems-rebuild`  
Scope: world generation, biome generation, world loading, loading presentation, persistence, and the external systems that consume world-generation queries.

## 1. Purpose

Asteria's current world stack has been rewritten and patched several times. Some surrounding systems are now good and must be preserved, but biome/world generation accumulated overlapping ownership, expensive hot-path repair logic, and contracts that leak into loading, persistence, structures, chat commands, warp, spawn, and dimension travel.

This rebuild is intentionally forward-only. It replaces the affected world systems instead of preserving obsolete internal behavior or save formats.

The rebuild must proceed in dependency order and in explicit phases. We do not start the next generation layer until the previous layer has a stable contract, tests, diagnostics, and acceptable performance.

## 2. Systems in rebuild scope

The following systems are expected to be replaced or substantially redesigned:

- surface/volume biome ownership and biome relationships;
- terrain/world generation;
- generated surface/material assignment;
- generated natural fluid placement that belongs directly to terrain/world formation;
- generation-side structure/feature integration;
- chunk synthesis from generated world data;
- world persistence contracts and serialized world state;
- initial world bootstrap/loading orchestration;
- loading progress model;
- loading screen presentation;
- worldgen query APIs used by commands, spawn, teleportation, portals, tools, and diagnostics.

## 3. Systems to preserve unless an explicit contract change requires adaptation

The rebuild must not casually replace working runtime infrastructure. Preserve these systems where possible:

- voxel/chunk runtime representation;
- chunk residency and streaming infrastructure;
- chunk rendering and visibility;
- meshing/remeshing;
- runtime lighting solver;
- runtime fluid simulation;
- generic Structure definitions, transforms, variants, groups, and connectors where their abstractions remain sound;
- player/gameplay systems;
- inventory, crafting, storage, entities, and unrelated content systems.

Preserved systems may receive narrow adapter/interface changes when the new world contract requires them. They are not automatically in rewrite scope.

## 4. Explicit non-goals and forbidden regressions

### 4.1 No dedicated hydrology subsystem

Do not recreate a hydrology graph, drainage network, river planner, basin system, or equivalent world-generation owner.

Generated water that is an immediate consequence of terrain/world formation may be produced by the terrain/material generation path. Rivers remain authored connected Structures/connectors according to `AGENTS.md`.

### 4.2 No `land biome` abstraction

Ocean is a biome, not a separate ownership universe. The new biome owner must not be structured as `ocean domain + land sites`, must not expose `land_biome_*` concepts, and must not invent separator biomes to repair adjacency.

### 4.3 No generation-order dependence

Generating a far location directly must produce the same result as reaching it through normal streaming. Output may not depend on which neighboring chunks were already generated, rendered, loaded, or requested first.

### 4.4 No hidden chunk generation from query commands

Queries such as biome location, spawn search, map visualization, structure location, and terrain sampling must not materialize entire chunk paths merely to answer a question.

### 4.5 No backward-compatibility burden by default

Old world-generation internals and old save formats do not constrain the new design. Migration/compatibility is out of scope unless explicitly requested later.

## 5. Cross-cutting contracts

These rules apply to every rebuild phase.

### 5.1 One authoritative owner per generated fact

Biome ownership, terrain shape, generated materials, structure placement, persistence state, and loading progress each have one authoritative owner. Consumers read those results; they do not rediscover them independently.

### 5.2 Pure deterministic queries

The generator must expose side-effect-free queries for information that does not require voxel materialization. Conceptually this includes capabilities such as:

- biome ownership/influence at world coordinates;
- surface/terrain information at world coordinates;
- biome search;
- deterministic structure-placement candidates;
- spawn suitability queries;
- map/debug sampling.

Exact APIs are decided during the foundation phase. Query calls must not silently mutate persistence, streaming, or chunk residency.

### 5.3 Query-based world model is authoritative

When working on this rebuild, implement the generated world as deterministic spatial fields that can be queried directly by world coordinate or bounded area. Do not introduce a fixed logical generation region, generation tile, or region ownership boundary into world semantics.

The runtime `VoxelChunk` remains a materialization/storage/rendering unit. It is not the semantic unit from which biome, terrain, or structure truth is derived.

Performance mechanisms are explicitly allowed and expected, but they must stay internal to the subsystem that owns the expensive query:

- bulk/area sampling;
- reusable per-request sample grids or snapshots;
- cache tiles;
- bounded LRU caches;
- spatial indexes;
- precomputed immutable metadata;
- batched noise/field evaluation.

Those mechanisms are implementation details only. Their dimensions, eviction order, task scheduling, cache warmth, or presence must never change generated output. A caller must not need to know that a cache tile or internal batch exists.

Do not implement the query-based contract naively by repeatedly invoking expensive scalar queries for every voxel when a chunk/area request can evaluate shared data once and reuse it across biome, terrain, material, and feature work. Query-based describes the ownership and external contract, not permission for redundant computation.

Changing an internal tile size, cache strategy, batching strategy, or execution order must preserve identical world results for the same seed, definitions, dimension, and coordinates.

### 5.4 Performance is a contract

Hot paths must be measurable. Expensive neighborhood searches, recursive repair, repeated noise evaluation, unbounded scans, and per-voxel rediscovery of large-scale fields require explicit justification and benchmarks.

### 5.5 Parallelism must not leak into semantics

Independent work may execute concurrently, but result order must not affect generated output or loading state correctness.

## 6. Implementation phases

### Phase 0 — Impact audit and external contracts

Before deleting old code, identify every subsystem that consumes biome/world-generation/loading/persistence behavior and document what capability it actually needs.

Known consumers to audit include:

- generic Structures and connector placement;
- `/locate biome`;
- `/locate structure`;
- `/warp`;
- spawn selection and respawn;
- portal/dimension travel, including the Dimensional Slicer path;
- chunk streaming and far-coordinate requests;
- biome visuals/tint consumers;
- world selection/new-world creation;
- persistence/save catalog/session reconstruction;
- loading screen and progress reporting;
- developer diagnostics and map/debug tools.

The audit must classify each dependency as one of:

1. preserved contract;
2. adapter required;
3. rewritten consumer;
4. obsolete dependency to remove.

**Gate:** we know which external capabilities the new world stack must expose before cleanup starts.

### Phase 1 — Cleanup

Remove obsolete world-generation/biome/loading/persistence ownership that would otherwise constrain the new architecture.

Goals:

- remove old biome-field ownership and repair/fallback paths;
- remove obsolete generation orchestration;
- remove obsolete loading-progress orchestration;
- detach obsolete persistence contracts from the active path;
- preserve only minimal compile-safe boundaries required by unrelated runtime systems;
- do not implement replacement behavior prematurely during cleanup.

**Gate:** old ownership cannot accidentally participate in later phases; branch remains buildable/testable at the intended intermediate level.

### Phase 2 — Generation foundation and query model

Define the minimal common foundation used by every later generation stage.

Responsibilities include:

- world seed and deterministic random/noise primitives;
- dimension generation context;
- world/chunk coordinate semantics;
- the query-based spatial contract defined in section 5.3;
- immutable generation snapshots/definitions;
- pure scalar and bounded-area query boundaries where each is appropriate;
- generation result/materialization boundary;
- explicit cache/batch ownership where measurement proves it is useful.

This phase must prove direct far-coordinate access: requesting data at B must never require materializing or traversing the path from A to B.

**Gate:** deterministic foundation tests, direct-coordinate query tests, and equivalence tests across different internal batching/cache conditions are green.

### Phase 3 — Biome Layout

Biome layout is the first world-content layer.

It must own:

- biome identity at surface/world positions;
- coherent biome region shapes;
- region sizing behavior;
- weights;
- adjacency/exclusivity/avoid-near semantics;
- conflict handling without generation failure;
- boundary representation;
- blend/influence information consumed by later systems.

Ocean participates as a normal biome identity.

Constraints such as min/max size, avoid-near, and exclusivity must not create an unsatisfiable planner that panics. Exact priority and conflict semantics remain an explicit design decision to settle before implementation of this phase.

#### Required biome map viewer

This phase is not complete without a real 2D biome-map visualization tool.

The viewer must be able to render a selected seed/dimension/area as a map-like image, not merely print sampled IDs. Required capabilities:

- biome colors/legend;
- configurable center and scale/zoom;
- configurable output resolution/area;
- biome boundary overlay;
- blend/influence overlay;
- ability to inspect region shape and small/sliver regions;
- deterministic output for a fixed seed/configuration;
- image export or equivalent persistent visual output for comparison.

Additional diagnostic overlays such as region IDs, constraint violations, region size, or repaired/absorbed areas should be added when they materially help validation.

**Gate:** biome maps are visually reviewable, deterministic, performant, and pass relationship/size/boundary tests before terrain begins.

### Phase 4 — Terrain

Terrain consumes the authoritative biome layout; it does not resolve biome ownership again.

Responsibilities include:

- base surface height/shape;
- mountains, plains, wastelands, swamps, ocean floors, floating formations, and other biome-authored terrain behavior;
- caves/overhangs/volume terrain where applicable;
- continuous cross-chunk world-space sampling;
- terrain blending driven by the biome boundary/influence representation.

**Gate:** terrain is deterministic across chunk seams and generation order, with visual/debug validation available before materials/features are layered on top.

### Phase 5 — Surface, materials, and generated natural fluids

Convert terrain into generated block/material composition.

Responsibilities include:

- surface and subsurface layers;
- stone/core layers;
- biome-driven material selection;
- snow/sand/etc. generated surface rules;
- ocean/body-of-water fill that is directly part of world formation;
- generated fluid placement without creating a separate hydrology owner;
- material/tint transitions derived from the same biome boundaries used by terrain.

Generated fluids must integrate with runtime fluid simulation through an explicit boundary; generation must not bulk schedule every generated fluid voxel as dynamic work.

**Gate:** surface/material/fluid output is deterministic, seam-safe, and does not introduce a second biome/boundary resolver.

### Phase 6 — Structures and features

Integrate the generic Structure/feature system after biome, terrain, and materials are authoritative.

Includes:

- trees and vegetation;
- boulders and surface objects;
- authored Structures/groups;
- connector-based chains;
- rivers through the generic Structure/connector architecture;
- waterfalls when represented as Structures/features;
- underground/volume features;
- cross-chunk placements without clipping at chunk boundaries.

Structure planning must be deterministic in world space. A structure crossing multiple chunks is one logical placement, not independently invented per chunk.

**Gate:** placement/location queries agree with materialized generation and remain generation-order independent.

### Phase 7 — Chunk synthesis

Define the authoritative conversion from generated world data into runtime `VoxelChunk` content.

The synthesis layer combines biome/terrain/material/feature results and publishes the minimal runtime data required by streaming, lighting, fluids, meshing, and rendering.

**Gate:** chunk output can be requested directly for arbitrary coordinates and matches fixed-seed deterministic snapshots/tests.

### Phase 8 — Consumer integration

Reconnect systems that need world knowledge to the new query/generation APIs.

#### `/locate biome`

Must query/search the biome layout without generating every chunk in the search area.

#### `/locate structure`

Must query deterministic structure-placement data without requiring full chunk materialization of the searched path/area.

#### `/warp`

Warp is destination relocation, not navigation through the world.

A warp must:

1. switch interest directly to the destination coordinates;
2. load/generate only the required destination neighborhood;
3. resolve a safe destination using the cheapest authoritative terrain/world information available;
4. never generate the spatial path between source and destination.

The current implementation already redirects streaming to the target instead of intentionally following the path, but its local safe-position search can become expensive. The rebuilt contract must avoid large brute-force 3D searches when terrain/surface information can narrow the candidate set.

#### Spawn and respawn

Spawn selection must use generation queries and intentionally request only the neighborhood needed to enter gameplay. It must not rely on generating a huge area and then discovering a safe position.

#### Portals and dimension travel

Portal travel, Dimensional Slicer travel, cross-dimension warp, and similar systems should share the direct destination-loading contract rather than each inventing a different transition mechanism.

#### Streaming

Streaming remains the runtime owner of residency/interest, but requests generation by coordinate through the new boundary and must not assume generation order affects output.

**Gate:** all audited consumers from Phase 0 are migrated or explicitly preserved through stable adapters.

### Phase 9 — Persistence

Design persistence only after generated-vs-mutable ownership is clear.

The new save model should persist authoritative mutable state, for example:

- seed/world/dimension/session metadata;
- player/session state;
- entities;
- inventories/storages;
- player/world edits or chunk deltas;
- runtime fluid/scheduled state when required;
- other state that cannot be deterministically reconstructed.

Do not persist generated caches, biome-map caches, meshes, derived lighting, generation queues, or other data that can be reconstructed from authoritative state unless measurement and correctness require a specific persisted cache.

**Gate:** new-format save/load roundtrip restores authoritative mutable state without making serialization shape the owner of world-generation semantics.

### Phase 10 — World loading pipeline

Rebuild world bootstrap/loading around explicit dependencies.

The pipeline must:

- distinguish required-to-enter-gameplay work from background continuation;
- expose structured progress independent of the loading UI;
- group concurrent work under a logical parent phase;
- support cancellation/stale-result rejection where relevant;
- avoid turning implementation workers into user-visible top-level phases.

**Gate:** initial world entry, load-from-save, dimension transition, and destination preparation use the same coherent progress model where appropriate.

### Phase 11 — Loading screen

The loading screen is a presentation consumer of the loading pipeline. It does not own world-loading logic.

It must show all meaningful steps, but parallel work is grouped under one logical phase instead of presenting many simultaneous top-level phases.

Example shape:

```text
Preparing World

Generating Initial Area
  Terrain        82%
  Structures     61%
  Finalization   74%

Preparing Presentation
  Lighting       55%
  Meshing        43%
```

The exact stages will be determined by the final pipeline. The invariant is hierarchical progress: logical phase first, concurrent child operations inside it.

**Gate:** progress accurately reflects the pipeline, never blocks on irrelevant background work, and remains stable when internal worker parallelism changes.

### Phase 12 — End-to-end validation and performance

Run fixed-seed, multi-seed, near/far-coordinate, save/load, dimension-travel, warp, locate, streaming, and visual validation.

Required performance coverage includes at least:

- biome query cost;
- biome-map generation cost;
- terrain query/generation cost;
- chunk synthesis cost;
- initial playable-area time;
- far-coordinate warp preparation;
- locate queries;
- peak temporary memory and cache growth where relevant.

**Gate:** correctness, determinism, visual review, targeted benchmarks, and CI are green before the rebuild is considered mergeable.

## 7. Current known affected subsystems

This list is intentionally explicit so cleanup does not accidentally delete a contract another subsystem still relies on.

| Consumer | Expected relationship to rebuild |
| --- | --- |
| Structures/connectors | preserve generic primitives, replace old worldgen coupling |
| Chat `/locate biome` | migrate to biome query/search API |
| Chat `/locate structure` | migrate to deterministic structure query API |
| Chat `/warp` | migrate to direct destination preparation + cheap safe-position query |
| Spawn/respawn | migrate to generation query API |
| Portals/Dimensional Slicer | use common direct destination-loading contract |
| Streaming | preserve residency/selection infrastructure; replace generator interface |
| Rendering/biome visuals | consume new biome influence/blend data; do not resolve biome ownership |
| Runtime lighting | preserve solver; adapt initial generated-content boundary only if required |
| Runtime fluids | preserve simulation; adapt generated-fluid frontier/publication boundary only if required |
| Persistence/save catalog/session | redesign serialized world contract around new authoritative state |
| Loading UI | replace current flat/concurrent stage presentation with hierarchical progress |
| Developer diagnostics | add biome map viewer and generation/performance diagnostics |

Phase 0 must expand or correct this table from current code before destructive cleanup begins.

## 8. Decisions to settle incrementally

### Resolved decisions

#### Spatial generation model — resolved 2026-10-04

The rebuild uses the query-based world model defined in section 5.3.

There is no fixed logical generation region. Generated facts are deterministic functions/fields of seed, definitions, dimension, and world coordinates. `VoxelChunk` remains the runtime materialization unit only. Internal cache tiles, batches, sample grids, and spatial indexes are allowed solely as bounded performance mechanisms and may not affect semantics or become visible dependencies of consumers.

### Open decisions

We will resolve these one at a time and update this document as decisions become authoritative:

1. exact pure query API and ownership;
2. biome min/max-size semantics;
3. `avoidNear` semantics and priority;
4. exclusive-neighbor semantics and conflict resolution;
5. biome blend representation and number of influences;
6. surface/volume biome relationship model;
7. ocean/sea-level terrain semantics without a hydrology subsystem;
8. terrain representation and cross-chunk sampling strategy;
9. structure planning/index/query contract;
10. safe spawn/warp destination query strategy;
11. generated vs persisted chunk/delta representation;
12. persistence format boundaries;
13. loading phase hierarchy and progress aggregation;
14. benchmark budgets and fixed-seed visual/performance fixtures.

A later implementation phase must not silently decide one of these differently from what the document records.

## 9. Branch and integration strategy

Implementation work for this rebuild lives on `world-systems-rebuild` until the new stack has an end-to-end playable path and passes its gates.

The branch may periodically incorporate current `main` changes so unrelated features are not lost. The rebuild should not be partially merged into `main` merely to reduce branch size.

Normal repository Git/CI rules remain defined by `AGENTS.md`; this document defines the rebuild architecture and phase order, not a competing Git policy.

## 10. Relationship to existing worldgen documents

`docs/design/worldgen-coherence.md` contains older deferred requirements and useful historical intent, but it predates this rebuild decision. During the rebuild, this document owns the active phase plan. Individual useful requirements from older documents must be revalidated against the new architecture before implementation rather than copied automatically.

Repository-wide normative rules in `AGENTS.md`, `ARCHITECTURE.md`, and `ENGINEERING_PRACTICES.md` still take precedence over this plan.