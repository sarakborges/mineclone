# Post-Refactor Worldgen Coherence Package

Status: **deferred feature package — implement only after the core rebuild is complete and green**  
Branch while planning: `architecture/asteria-core-rebuild`  
Scope: surface/volume biome relationships, terrain transition invariants, volume-biome surface indicators, and floating-island formation.

## 1. Intent

This package groups related world-generation changes that should land after the architecture refactor instead of being mixed into the migration itself.

The shared goal is to make biome relationships and terrain formation explicit, deterministic, data-driven, and testable. The implementation must use the post-refactor world metadata/generation boundaries rather than reintroducing special-case orchestration.

The package has four workstreams:

1. ocean margins are never allowed to raise terrain;
2. volume biomes can constrain which surface biomes they may coexist with;
3. a volume biome can optionally advertise its presence through a surface structure;
4. floating islands are generated as coherent islands with proper terrain stratification rather than disconnected stone blobs.

## 2. Cross-cutting rules

These rules apply to all four workstreams.

### 2.1 Data-driven relationships

Biome relationships belong in biome/worldgen metadata, not in scattered `if biome == ...` branches.

Prefer semantic selectors that can address:

- an exact biome ID;
- a biome family;
- biome tags/classes;
- combinations of the above where needed.

This is important for rules such as “all mountain variants” without requiring every new mountain biome to be added manually to terrain code.

### 2.2 Deterministic world-space sampling

Any terrain/biome decision that may cross chunk boundaries must be sampled from stable world coordinates and world seed data.

The output must not depend on:

- chunk generation order;
- which neighboring chunks are currently resident;
- presentation state;
- frame timing;
- ECS entity existence.

### 2.3 Explicit pipeline ownership

The post-refactor pipeline should make the relevant dependency order explicit:

```text
surface biome field
        |
        +--> base terrain shape
        |
        +--> volume-biome eligibility / placement
                    |
                    +--> volume-biome metadata
                              |
                              +--> structure planning / optional surface indicator

terrain transition policies --> final terrain shape
floating-island formation ----> terrain/material columns
```

Exact internal types may evolve during implementation, but the dependency direction must remain explicit and testable.

### 2.4 No presentation coupling

None of these decisions may depend on rendered chunks, mesh state, cameras, visibility, or presentation residency.

## 3. Workstream A — Ocean margins are lower-only

### 3.1 Requirement

Ocean/coastal influence must never raise the terrain height it receives from the preceding terrain-shaping stages.

This is a general invariant, not a mountain-specific patch:

```text
height_after_ocean_margin <= height_before_ocean_margin
```

As a consequence, ocean margins cannot lift the edges/foothills of mountains of any kind.

### 3.2 Desired behavior

Ocean margins may:

- lower terrain toward sea level;
- carve or soften a coast;
- truncate a mountain that reaches the coast;
- attenuate terrain height according to the configured coastline profile.

Ocean margins may not:

- add positive height;
- create a ridge where the pre-ocean terrain was lower;
- pull mountain foothills upward toward a blend curve;
- require a hardcoded list of mountain biome IDs to avoid the problem.

### 3.3 Implementation direction

Represent the rule as an explicit terrain-transition contract/policy, for example a lower-only height influence. The exact API is deliberately left to the post-refactor implementation, but the semantic rule must be enforceable in one place.

A safe composition rule is conceptually equivalent to:

```text
final_height = min(pre_ocean_height, ocean_shaped_height)
```

This example is not a requirement to use `min` literally; it defines the invariant the final implementation must preserve.

### 3.4 Acceptance criteria

- Ocean-margin processing never increases a sampled height.
- The property holds for every mountain family/variant without per-variant terrain code.
- Coastlines do not develop an artificial raised rim solely because of ocean blending.
- The same seed and world coordinate produce the same result regardless of chunk generation order.
- Existing inland mountain profiles remain unaffected by this rule.

## 4. Workstream B — Volume-biome restrictions by surface biome

### 4.1 Requirement

A volume biome must be able to declare which surface biomes it is compatible or incompatible with.

Example semantic shape:

```text
VolumeBiomeDefinition
  surface_constraints:
    allow: <optional surface-biome selector>
    deny:  <optional surface-biome selector>
```

The final serialization/API may differ, but it must remain data-driven.

### 4.2 Selector semantics

Selectors should support at least:

- exact surface biome IDs;
- biome family/tag selectors.

Recommended evaluation semantics:

- no `allow` selector means “allowed anywhere unless denied”;
- `deny` takes precedence over `allow`;
- an empty constraint block preserves current unrestricted behavior;
- constraints are evaluated against authoritative surface-biome metadata, not rendered terrain.

This allows rules such as:

- a volume biome only beneath cold surfaces;
- a volume biome forbidden beneath oceans;
- a volume biome allowed beneath an entire mountain family;
- future surface-biome variants inheriting behavior through tags/families instead of code edits.

### 4.3 Pipeline rule

Surface-biome classification must be available before the final volume-biome eligibility decision for the same world column/region.

The implementation may still calculate fields in parallel where dependencies permit, but the semantic result must be equivalent to filtering volume-biome candidates through their surface constraints.

### 4.4 Acceptance criteria

- A constrained volume biome appears under allowed surface biomes.
- It never appears under explicitly denied surface biomes.
- `deny` wins when a surface matches both selectors.
- Family/tag selectors cover variants without enumerating IDs.
- Unconstrained volume biomes preserve unrestricted behavior.
- Boundaries remain deterministic across chunk seams and generation order.

## 5. Workstream C — Optional volume-biome surface indicator

### 5.1 Requirement

A volume biome may optionally define a structure that can appear in the surface biome to indicate that the volume biome exists below/near that location.

This is a generic biome-to-structure relationship. It must not be implemented as special logic for one cave/volume biome.

### 5.2 Data shape

Conceptual metadata:

```text
VolumeBiomeDefinition
  surface_indicator: optional
    structure_group: <id>
    relation: <placement relation>
    horizontal_radius: <range>
    depth_range: <range>
    spacing: <rule>
    chance: <rule>
    surface_filter: <optional selector>
```

The exact fields should be reduced to what the implementation actually needs, but the system must support deterministic placement rules and a structure group rather than requiring a single authored variant.

### 5.3 Placement semantics

The structure planner should be able to query authoritative volume-biome metadata without requiring the volume to be rendered or materialized first.

Conceptual flow:

1. resolve surface biome metadata;
2. resolve volume-biome presence/occupancy;
3. structure planning asks whether the configured volume biome exists within the indicator relation/range;
4. if present, choose a valid surface anchor;
5. apply normal structure validity/spacing rules;
6. place one deterministic structure/group choice when eligible.

A failed surface placement must simply skip the indicator. It must not change the underlying surface or volume biome assignment.

### 5.4 Rules

- No indicator without the associated volume biome actually being present in the configured relation/range.
- The indicator does not change the identity of the surface biome.
- Normal surface structure constraints still apply.
- Neighboring chunks must not independently duplicate the same logical indicator.
- Structure-group selection must remain deterministic.
- The system should support authored variants through an existing/new generic structure group rather than biome-specific branching.

### 5.5 Acceptance criteria

- Positive case: valid volume biome + valid surface anchor can produce the configured indicator.
- Negative case: no matching volume biome means no indicator.
- Invalid surface placement skips cleanly without mutating biome metadata.
- Cross-chunk planning does not duplicate one logical placement.
- Same seed produces the same indicator location and variant regardless of chunk order.

## 6. Workstream D — Floating islands as coherent islands

### 6.1 Requirement

Floating islands must read as actual landmasses, not randomly scattered pieces of stone.

They should be larger, varied, irregular, and organically shaped while preserving gradual transitions between the parts/lobes that compose an island.

### 6.2 Formation model

Replace independent small-blob composition with a coherent world-space island field.

Recommended shape hierarchy:

1. **Macro footprint** — establishes one large connected island region.
2. **Irregular silhouette** — low-frequency deformation/domain warping breaks circularity.
3. **Secondary lobes** — broad overlapping forms add bays, protrusions, asymmetry, and size variation.
4. **Smooth union/blending** — lobes transition gradually instead of creating abrupt seams or disconnected rubble.
5. **Thickness envelope** — island thickness decreases smoothly toward its footprint boundary.
6. **Surface relief** — gentle top-surface variation adds terrain character without breaking landmass continuity.

The exact noise/SDF implementation is open, but all samples must be derived from deterministic world coordinates so a single island can cross chunk boundaries seamlessly.

### 6.3 Connectivity rules

Default floating-island generation should produce a coherent primary landmass.

- Tiny disconnected components below a configurable threshold are rejected.
- Accidental “stone confetti” is not valid output.
- Intentional satellite islands may be supported later as an explicit profile/feature, but should not emerge accidentally from the base generator.
- The generator should favor broad connected forms before adding silhouette detail.

### 6.4 Terrain stratification

Floating islands use terrain layers, not a uniform stone volume.

From the exposed top downward:

```text
grass / configured surface block
        |
        v
dirt / configured subsurface layer
        |
        v
stone / configured core material
```

Rules:

- exposed walkable top surfaces receive the surface block (grass by default);
- a configurable dirt/subsurface depth sits below the top;
- the remaining interior is stone/core material;
- steep sides and the underside naturally expose core material where there is not enough vertical depth for the topsoil stack;
- material choice must be column/surface aware rather than a post-process that paints every exposed face as grass.

### 6.5 Shape diversity

Island profiles should permit variation in at least:

- overall horizontal scale;
- aspect ratio;
- thickness;
- lobe count/strength;
- silhouette warp;
- top relief;
- edge taper.

Variation must remain within coherent shape bounds: diversity is not an excuse to reintroduce disconnected random blobs.

### 6.6 Acceptance criteria

- Generated islands have a recognizable coherent primary landmass.
- Islands are materially larger than the current small floating stone fragments.
- Silhouettes vary across seeds/locations and are not simple repeated circles.
- Composed lobes blend gradually without visible hard seams.
- Edge thickness tapers gradually instead of terminating as a uniform plate.
- Tiny accidental detached stone components are removed/rejected.
- Top terrain follows surface -> subsurface -> core stratification.
- Same seed/world coordinate is deterministic.
- Islands cross chunk boundaries without seams or generation-order dependence.

## 7. Shared primitives to implement first

Before the four feature paths diverge, add/reuse the smallest generic primitives required by the package:

1. semantic biome selectors (ID/family/tag);
2. reusable surface-biome constraint evaluation;
3. authoritative volume-biome occupancy/proximity query usable by worldgen/structure planning;
4. explicit terrain-transition influence contract capable of enforcing lower-only behavior;
5. deterministic world-space sampling helpers for coherent multi-chunk formations.

Do not create generic abstractions that have no concrete consumer in this package. The goal is to remove special cases, not to build a speculative framework.

## 8. Planned implementation order after the refactor

### Slice 1 — biome relationship primitives

- add semantic selectors/tags needed by actual biome definitions;
- implement/test allow/deny evaluation;
- expose stable surface-biome metadata to volume-biome eligibility.

**Gate:** unit tests + deterministic boundary tests green.

### Slice 2 — volume-biome surface restrictions

- wire constraints into volume-biome candidate resolution;
- migrate/add definitions that need restrictions;
- add regression coverage for unrestricted volume biomes.

**Gate:** no forbidden surface/volume combinations in seeded test fixtures.

### Slice 3 — volume-biome surface indicators

- expose the required occupancy/proximity query to structure planning;
- add optional `surface_indicator` metadata;
- support structure-group placement and cross-chunk deduplication;
- add one representative definition only after the generic path is proven.

**Gate:** indicator correlation/deduplication/determinism tests green.

### Slice 4 — ocean lower-only margin invariant

- move ocean/coastal shaping behind an explicit transition policy;
- guarantee non-positive ocean height influence;
- cover all mountain families through the invariant rather than per-biome exceptions.

**Gate:** property/regression tests prove ocean shaping never raises terrain.

### Slice 5 — floating-island formation rewrite

- replace random stone-fragment composition with coherent island-field generation;
- implement macro footprint, smooth lobe blending, thickness taper, and shape variation;
- add surface/subsurface/core material stratification;
- reject tiny accidental detached components.

**Gate:** connectivity, layering, seam, and deterministic-seed tests green.

### Slice 6 — integration tuning

- run a multi-seed/multi-region validation matrix;
- inspect ocean/mountain boundaries, constrained volume biomes, surface indicators, and floating islands together;
- tune data values without weakening invariants;
- profile generation cost and ensure no unbounded cross-chunk searches were introduced.

**Gate:** CI green, deterministic tests green, no new unbounded queues/searches, and visual/gameplay validation accepted.

## 9. Required tests

At minimum, add coverage for:

- `SurfaceBiomeSelector` exact ID/family/tag matches;
- allow/deny semantics and deny precedence;
- unrestricted volume-biome backward behavior;
- ocean-margin property: `height_after <= height_before`;
- multiple mountain variants against the ocean-margin invariant;
- volume-biome surface-indicator positive/negative cases;
- structure-indicator deduplication across neighboring chunks;
- generation-order invariance for all cross-chunk decisions;
- floating-island connected-component threshold;
- floating-island grass/dirt/stone column stratification;
- floating-island seam continuity across chunk boundaries;
- deterministic snapshots/sampled hashes for fixed seeds where appropriate.

## 10. Non-goals

This package does **not** include:

- reintroducing the removed legacy hydrology system;
- retuning every existing biome;
- adding unrelated biome IDs merely to exercise the framework;
- changing runtime presentation/meshing behavior;
- preserving obsolete world-generation quirks for old saves;
- implementing these features before the current architecture rebuild is complete and green.

## 11. Definition of done

The package is complete when:

- ocean/coast shaping has a tested lower-only terrain invariant;
- volume biomes can declaratively allow/deny surface-biome relationships;
- volume biomes can declaratively expose an optional deterministic surface structure/group indicator;
- floating islands form large coherent irregular landmasses with gradual shape transitions and surface/subsurface/core stratification;
- all behavior is deterministic across seeds, chunk boundaries, and generation order;
- no new biome-specific orchestration is required for these capabilities;
- CI and the package-specific regression suite are green.