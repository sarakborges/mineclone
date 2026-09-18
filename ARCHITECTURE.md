# Asteria Architecture Canon

This document defines the architectural rules that new code and refactors in Asteria must preserve. It is intentionally concrete: when a shared primitive or context already exists, new features should extend it instead of recreating the same behavior locally.

## 1. Single authoritative owner

A gameplay fact must have one authoritative owner.

- Do not mirror a `State<T>`, resource, registry, queue, selection, or world value into a second boolean/resource just to make another system easier to write.
- Derived read contexts may combine authoritative sources, but they must not become a second source of truth.
- `WorldInteractionState` is the canonical example: it reads the authoritative game, pause, settings, inventory, and brush-palette states and derives whether world interaction is available.
- If a value can be recomputed cheaply and deterministically from authoritative state, prefer recomputing or caching it locally over introducing persistent duplicate state.

## 2. Reuse invariants, not superficial similarity

Code should be shared when multiple callers implement the same invariant, transformation, state machine, queue mechanic, or visual primitive.

Do not merge systems merely because they have similar shapes. Systems with different lifecycle semantics, side effects, or ownership should remain separate and call a shared lower-level primitive instead.

Canonical examples:

- `NumericInputState` owns numeric editing behavior; Settings sections only resolve and apply their domain value.
- `set_visibility` owns generic visibility mutation; lifecycle-specific systems decide when it should happen.
- `reset_resource` owns full `Default` resets; domain-specific partial resets remain explicit.
- `FrameWorkBudget` owns per-frame time/item budgeting; streaming, loading, unload, and remesh own the work being budgeted.
- `CurrentBiomeVisuals` owns weighted biome visual sampling; rendering systems choose which visual property to blend.

Avoid wrappers that only call another helper without adding domain semantics.

## 3. `SystemParam` contexts

Use `SystemParam` to name coherent read/write contexts and to keep Bevy system signatures below practical limits.

- A context should represent one domain concern, not a generic bag of unrelated resources.
- Compose smaller contexts when one context is a strict subset of another. `ChunkContent` composing `VoxelContent` is the model to follow.
- Reuse existing contexts such as `CurrentDimensionContext`, `BlockTargetingScene`, `BlockVisualContent`, and the chunk contexts instead of redeclaring the same resource cluster.
- Keep mutable contexts narrow. Broad mutable access makes scheduling conflicts harder to reason about.
- Bevy 0.19 systems have a technical limit of 16 system parameters, but code should keep comfortable architectural margin instead of treating 16 as a target.
- As a practical guideline, keep systems and substantial helpers at roughly 8–12 top-level parameters or fewer. Around 10 meaningful dependencies, review ownership and cohesion before adding another dependency.
- The same dependency discipline applies to helpers. Moving an oversized system signature into a giant helper does not resolve the architectural problem.
- Group dependencies only when they form a real operation or domain context. Do not introduce a `GodParam`, `GameContext`, or other generic bag merely to lower the visible count.
- Schedule tuples must remain within the 20-item tuple limit. Split large orchestration into sets or coherent subgroups before reaching the limit.

## 4. States, availability, and run conditions

Gameplay-input systems must use the canonical availability rule instead of rebuilding state chains locally.

- Use `world_interaction_available` for actions that are valid only while the player can interact with the world.
- `WorldInteractionState` currently requires Gameplay, running pause state, closed Settings, closed Inventory, and closed Brush Palette.
- When a new modal state blocks world interaction, update the canonical availability context first rather than patching every consumer independently.
- Systems that must keep draining input/events while blocked may run outside that condition, but must explicitly avoid applying gameplay effects. Mouse-look event draining is the canonical example.
- Systems that own stale derived state must clear it when interaction becomes unavailable. Targeting must clear `TargetedBlock`, not merely skip raycasting.
- Modal HUD visibility must derive directly from authoritative states, not from one-shot transition handlers or `State::is_changed()` shortcuts. Gameplay HUD roots that must disappear under Pause/Settings should reconcile their `Visibility` idempotently while Gameplay is active, and must tolerate zero or multiple matching roots during lifecycle edges.

Use schedule sets to express ordering between domains. Prefer named sets over chains that only exist to compensate for hidden side effects.

## 5. UI design system ownership

Reusable UI behavior and appearance belong under `src/ui`.

- Screens and HUD modules resolve domain data and assemble layouts; they should not clone interaction mechanics or control styling.
- View-only state such as search focus, filter text, selected presentation categories, scroll positions, and UI dirtiness belongs to the owning HUD/screen module rather than gameplay/player domains.
- Action buttons use the canonical `ui::button::button(...)` primitive and `ButtonVariant`; screens must not create parallel button constructors for the same interaction model. Canonical button labels render in Title Case (initial uppercase for every whitespace-separated word), so screens/localizations must not invent a different capitalization rule.
- Numeric fields use `numeric_input_field`, `NumericInputState`, and `sync_numeric_input_view`.
- Editable fields reuse `ui::text_input` for input surface, cursor/text styling, padding, and focus-border rules while domain validation and focus lifecycle stay with the owning screen/HUD.
- Selectable control states (normal, hover, pressed, selected, danger) belong to `ui::selectable`; `ui::surface` owns containers/panels and must not absorb interactive-control state again.
- Dropdown shell geometry, option visuals, open/close state, chevron, and outside-click detection belong to `ui::dropdown`. Every interactive descendant that belongs to an open dropdown — including search controls, option lists and scrollbars — must participate in the typed `DropdownInside<M>` area so internal clicks are never misclassified as outside clicks. Scrollbars embedded in dropdowns must mark both track/root and thumb via the typed scrollbar variant; marking only the parent is insufficient because `Interaction` does not propagate from the clicked thumb. Search/filter semantics, option data, and domain side effects remain with the owning screen.
- Toggle geometry and visual states belong to `ui::toggle`; domain resources own the boolean value being toggled.
- Shared screen header/body/footer geometry belongs to `ui::screen`; screens should compose their domain content inside that chrome instead of copying the same dimensions/padding locally.
- Shared settings rhythm belongs to `ui::settings`: use the canonical gap between setting groups separately from the smaller gap inside one setting.
- Shared typography, shadows, scrollbar behavior, transitions, and visibility helpers remain centralized.
- New controls that repeat an existing visual/interaction invariant in two places should become a design-system primitive before a third copy appears.
- Do not create a generic UI abstraction when the only similarity is a few `Node` fields and the controls have different behavior.

## 6. Targeting and block display

Targeting has one raycast result and downstream consumers observe it.

- `TargetedBlock` is the authoritative current block target.
- A creature with dead `EntityHealth` is not a valid `TargetedCreature`; dead entities must leave interaction targeting immediately even if their despawn/death animation is still pending.
- `BlockTargetingScene` is the shared read context for target, hotbar selection, player transform, and voxel world.
- Placement orientation, interaction, highlight, placement preview, and target HUD must derive from the same target/selection sources rather than maintaining independent copies.
- Shared block model/material/tint/orientation transforms belong in rendering/block-model utilities. Hotbar icons, held blocks, target icons, and placement previews may have different lifecycles but should reuse those transformations.
- Creature GLB node transforms are authored asset data and must not be reset by the generic visual loader. JSON texture/tint overrides may replace material inputs, but texture-only materials must preserve the GLB's authored alpha mode; only explicitly tinted body materials are forced opaque.
- Block display consumers that need asset, block, biome, and biome-field reads should reuse `BlockVisualContent` instead of redeclaring that cluster locally.

## 7. World queues and frame budgets

Queue mechanics must be layered rather than copied.

- `DeduplicatedQueue<T>` owns generic deduplicated FIFO/priority behavior.
- `VoxelUpdateQueue` adds voxel-domain rules such as nonnegative Y and neighbor expansion.
- Lighting/fluid/remesh queues add only their domain-specific behavior on top.
- Do not implement another `VecDeque + HashSet` pair locally for the same semantics.

Time-sliced world work uses `FrameWorkBudget`.

- The caller selects duration, minimum guaranteed items, and optional maximum items.
- The caller still owns what counts as a processed item.
- Do not hand-roll additional `Instant::now() + processed + elapsed` loops when the same semantics are required.

Natural hydrology is generated authoritatively with terrain/world generation. It must not be bulk-enqueued into the runtime dynamic-fluid solver. Dynamic fluid updates are for runtime topology/fluid changes.

Biome hydrology permissions apply to the full physical feature, not only to its source cell. In particular, `canGenerateRiver=false` forbids river channels from traversing that biome: downstream selection must stop at disabled land, immediate edges into disabled land must be rejected, and the continuous curved river path must be sampled and rejected if it crosses disabled surface space. Physical ocean destinations remain valid even when their underlying surface biome disables rivers.

Hydrology material ownership is biome-local. `riverBedBlock`, `lakeBedBlock`, `shoreBlock`, and `oceanBedBlock` belong to `BiomeHydrology`, not `DimensionHydrology`. River/lake/ocean-bed/shore materials resolve from the owning surface biome definitions. `DimensionHydrology` may reference the Ocean surface-biome ID and own the global water fluid plus generation weights, but hydrology must never replace the selected surface-biome identity or own terrain block materials. Coast is not a biome identity.

Fluid behavior is definition-owned. Color, opacity, roughness, metallic response, light dampening, colored `lightEmission`, `spreadSpeed`, and `maxSpread` belong to `FluidDefinition`; world systems must not special-case water or lava IDs for those behaviors. Fully opaque fluids render with opaque material semantics instead of alpha blending. Fluid emission uses the fluid's authored HSI color and scales with fill level before entering the normal block-light propagation pipeline.

Runtime dynamic-fluid simulation uses one gravity-first discrete voxel solver for every fluid. Vertical descent has priority and resets `spread_distance`, so each landing starts a fresh horizontal run bounded by that fluid's authored `maxSpread`. Dynamic falling cells do not spread sideways while airborne. On supported terrain, horizontal flow searches only within its remaining `maxSpread` for the nearest reachable downward opening and prefers first-step directions on a shortest path to that opening; if no drop is reachable in-range, it falls back to ordinary radial spreading. This routing must remain fluid-ID agnostic and must not bulk-enqueue deterministic natural hydrology.

Surface biomes may author an optional `surfaceFluid` generation rule. Such rules are terrain-shape features, not hydrology overlays, and must reference registered fluids. The current `volcano_crater` rule is valid only with Volcano terrain: it derives a flat crater-fluid level from the authored volcano geometry and may add sparse deterministic spill channels on the outer slope. Generation branches on the rule/terrain type, never on a biome ID.

Surface biomes may also author an optional `surfaceMargin`. A surface margin is a boundary modifier owned by the source biome, not a biome identity and not a second territorial selection. Its authored `width` is measured from the actual warped Voronoi boundary to a neighboring surface region; on the neighboring side it may replace surface material layers while the neighboring biome remains the authoritative `CurrentBiome`. Ocean uses this mechanism for shoreline/beach material, so a beach beside Witchwood remains Witchwood and a beach beside Plains remains Plains.

Every physical surface formation is a true `Surface` biome participating in one territorial surface-region selection. Plains, Wasteland, Witchwood, Enchanted Forest, Mountains, Gorge, Alps, Mountain Belt, Volcano, and Ocean may not be applied afterward as identity/terrain overlays that consume or shrink another biome. Edge treatments such as Ocean shoreline margins are boundary modifiers rather than physical formations or biome identities. Mountain-family biomes each own their distribution, `BiomeTerrain` generator, surface layers, hydrology permissions, structures, visuals, and surface-carver permission. Non-regional distributions influence whether/how strongly a biome competes for a surface site; after a biome wins that region, its distribution strength may still shape internal geometry such as Gorge depth or Volcano cone/crater form, but it may not overwrite a neighboring region. No terrain generator may branch on a hardcoded biome ID. Volcano explicitly disables river and lake generation and uses `asteria:bassalt` as its authored surface material.

Ocean is a surface-region identity, while shoreline and ocean water/floor carving are boundary/hydrology geometry. The Ocean-authored `surfaceMargin` provides the land-side shoreline material without replacing the neighboring surface identity. Hydrology may only express physical ocean in proportion to selected Ocean surface influence; low continentalness alone must never carve an ocean through an unrelated land biome. Open ocean receives deterministic multi-scale bathymetric relief that fades through the shoreline transition. Density carving and physical-water queries must call the same ocean-floor target so the carved bed and reported water bed can never diverge.

Lake geometry is authoritative where a river enters or crosses a lake. River channel carve and river headroom must fade continuously toward zero as supported lake opening strength approaches 1, so the lake basin is not re-cut by an independent river trench. River banks may still open the lake shore at the transition, but lake interior density owns the final basin shape.

Deterministic worldgen output is derived state until gameplay mutates that chunk. An unmodified generated chunk may be discarded completely when it leaves streaming retention and regenerated from the same seed when revisited. The first persistent block/fluid mutation makes the chunk authoritative saved state; such chunks are retained/archived and serialized. Loaded legacy/saved chunks are treated conservatively as persistent because the save format does not distinguish authored edits from generated content. Meshes and lighting remain derived and may be rebuilt.

Chunk persistence must scale with authoritative edited state, not distance explored. Disk chunks use state palettes plus contiguous voxel runs rather than repeating full block/fluid state per occupied voxel; legacy per-voxel snapshots remain readable. Periodic autosave must never clone the resident `VoxelWorld`: the main thread captures a serializable `WorldSnapshot` containing metadata plus persistent chunks only, and a detached task validates/serializes/publishes it. Only one autosave publication may be in flight per world session. Final Leave World/Exit commits remain synchronous so the world is not abandoned before durable publication.

Land cave-surface ownership is singular: the anchored cave connector graph must not create its own terrestrial surface entrance. On land, only `surfaceCarver` owns the visible cave mouth; ocean cave openings remain a separate hydrology-driven exception. Cavern volume biomes author the entrance `surfaceCarvers` in data, while each surface biome independently opts into receiving those cuts with `allowSurfaceCarvers`. A surface biome with the flag disabled is a hard ownership boundary: terrestrial cave mouths must not be carved through it. A surface tunnel candidate must identify an actual terrain mouth, then keep only the corridor from that mouth to the first sufficiently deep intersection with the anchored cave connector graph. Tunnel `elevation` data is an absolute underground world-Y target, never an offset added to sea level; the mouth Y comes from the actual terrain surface, and the authored path descends from that mouth toward the target before connectivity is accepted. A shallow graph touch is not a valid connection, and any path after the first deep connection is discarded, preventing surface dead ends and stray cuts that go nowhere. Structure-support sampling must use the same resolved corridor. Surface-tunnel geometry must preserve a visibly curved centerline at authored lengths; do not approximate long paths with a handful of tens-of-block straight capsules. Near the terrain surface, the 3D tunnel is not allowed to define the hillside shape by itself: the mouth owns a lake-style 2D surface basin. The center of the core grades terrain toward the tunnel interior, the core boundary converges to the tunnel roof, and a world-space outer margin then fades with smoothstep back to natural terrain. Deep underground the profile remains circular. Candidate reach/culling and structure-support sampling must use this same resolved basin geometry.

Spawn-column dryness must use the same physically supported hydrology as terrain generation. When actual terrain surface height is available, bootstrap/spawn selection must use `supported_water_at(...)` rather than unfiltered `water_at(...)`, so unsupported lake/ocean candidates cannot make genuinely dry terrain impossible to select.

A user-selected Spawn Biome is not a request to search the seed for a distant natural occurrence. It is a deterministic initial-region override owned by `BiomeField`, centered on the default spawn. The forced region must respect the selected dimension biome's authored `size.x/z.min..max`: deterministic radii are chosen inside those ranges, the core is elliptical rather than chunk-aligned, and the border uses the same surface warp/fade machinery as biome sampling. The override feeds terrain sampling and biome identity. Forced land biomes suppress underlying ocean strength only as much as the forced-biome weight requires; forced Ocean must preserve oceanic continentalness rather than suppressing itself. Only dry-column placement is searched when the selected spawn contract requires dry terrain. The selected spawn biome is persisted in world snapshots and restored on load so the same region is reconstructed across sessions.

Surface-biome `avoidNear` means exactly that two conflicting surface regions may not share a boundary. It applies equally to ordinary regional biomes and non-regional mountain formations after both participate in the same site-selection graph. It is not an exclusion radius, distance check, dominant-neighbor heuristic, overlay suppression, or rarity control. Site selection keeps the normal raw weighted biome whenever its Voronoi region does not share an edge with a conflicting raw neighbor; only a real shared Voronoi border may trigger a replacement. The rule is symmetric through `biomes_conflict`, and replacement must never reintroduce a forbidden shared border. Frequency remains controlled by weights/climate/distribution parameters.

Surface-biome `requireNear` is the complementary data-driven adjacency rule. A biome with a non-empty list may win a surface region only when that region shares a real Voronoi border with at least one listed surface biome (OR semantics). It is not a radius or climate hint. Required targets must exist in the same dimension, be active `Surface` biomes, and may not simultaneously appear in `avoidNear`. Biomes that require another biome cannot be selected as a standalone forced Spawn Biome, because that override would violate their authored adjacency contract. Shorelines do not use `requireNear`; they are modeled as `surfaceMargin` boundary modifiers instead.

## 8. Rendering and color

Semantic/internal color is HSI-first.

- Use `content::color::Hsi` for authored, blended, and semantic color operations.
- Convert to RGB/sRGB only at unavoidable rendering, GPU, image, material, or external-API boundaries.
- Weighted biome visuals should use `CurrentBiomeVisuals` instead of independently resolving `CurrentBiome.influences` against `BiomeRegistry`.

Rendering systems should avoid rebuilding expensive material/model/tint state every frame when the authoritative inputs have not changed. Use Bevy change detection, snapshot/local caches, spatial-cell caches, or event-driven refreshes as appropriate.

## 9. Lifecycle helpers

Prefer the existing generic lifecycle primitives when the operation is truly generic.

- Full `Default` reset: `reset_resource::<T>`.
- Visibility toggle by marker: `set_visibility::<M, VALUE>`.
- State close/reset helpers: use the canonical helpers in `app::state_systems`.

Do not replace a partial reset with `reset_resource` if doing so would erase unrelated persistent state. Hotbar selection reset is intentionally domain-specific because resetting `PlayerHotbar` would also erase inventory contents.

## 10. Boundaries and module ownership

A module boundary should own meaningful semantics.

- Remove modules/functions whose only purpose is forwarding to another generic helper.
- Keep modules that define a domain lifecycle, invariant, data model, or substantial transformation even if their code is small.
- Shared primitives should live at the lowest layer that understands their semantics. Do not move domain policy into generic utility modules.
- Avoid circular ownership: UI reads gameplay state; gameplay should not depend on HUD implementation details. Rendering utilities may be reused by HUD/viewmodel/preview, but should not know those consumers.

## 11. Performance expectations

Asteria targets stable 60 FPS and world streaming must protect frame time.

- Expensive work must not be repeated every frame without a demonstrated need.
- Streaming/generation/remesh/lighting work that remains synchronous must be explicitly budgeted.
- Prefer change-driven updates and caches for UI/model/material refreshes.
- Rebuild only the smallest stable UI/render subtree whose authoritative inputs changed; preserve unaffected roots, controls, slots, and materials.
- Derived metadata from loaded definitions belongs to the owning definition or registry. Precompute immutable voxel expansions, bounds, capability flags, sorted lookup lists, and similar summaries during load/insert instead of rescanning or reparsing definitions in generation, rendering, or UI hot paths.
- If definitions may replace an existing ID, rebuild derived metadata from authoritative definitions so cached summaries remain exact rather than monotonic or stale.
- Avoid broad neighbor remeshes when a boundary/content test can determine whether work is necessary.
- Do not trade away correctness of authoritative world data to hide a performance problem. Move or stage expensive work instead.

### Async world pipeline

Heavy generation and initial meshing belong off the main frame when they can operate on immutable inputs. The canonical pipeline is:

`generation task → integrate chunk → initial lighting seed → halo snapshot → mesh task → spawn render entities`

- Background tasks operate on immutable snapshots or task-owned data. They must not carry `Commands`, mutable `Assets`, mutable material handles, or an ECS `World` for off-thread mutation.
- Inputs that can change while a task is in flight must be versioned or otherwise revision-tracked. Task results are valid only for the authoritative input revision they were built from.
- Stale generation or mesh results must be discarded and, when the work is still required, rescheduled from current authoritative inputs.
- Snapshot boundaries must contain everything the task needs, including neighbor/halo data required for culling, ambient occlusion, lighting, and fluid-surface decisions. A task must not reach back into mutable runtime world state.
- Integrating a completed task is main-thread work and must respect frame budgets. Moving CPU construction off-thread does not justify unbounded result integration, entity spawning, asset insertion, lighting, or remesh work in one frame.
- Updates that arrive while work is in flight must not be lost. Remesh, lighting, fluid, or topology invalidation remains pending until the authoritative result reflecting that update has been integrated.
- Generation and runtime simulation remain separate responsibilities. Natural rivers, lakes, and oceans are authored by deterministic generation rather than reconstructed by the runtime fluid solver.
- Initial direct-light seeding may remain synchronous when required to preserve approved lighting semantics, but its frame cost must stay explicit and budgetable.

## 12. Versioning and commits

The root `VERSION` file is the canonical application version.

- Every completed coherent update block must bump `VERSION` according to semantic versioning: patch for fixes/refactors without feature-level behavior expansion, minor for backward-compatible feature blocks, major for incompatible architectural/product changes.
- Do not use `Cargo.toml` as the application-version authority.
- Keep commits small and coherent enough that architectural changes can be reviewed and reverted independently.
- Work is performed on `develop` unless a different branch workflow is explicitly requested.

## 13. New-feature checklist

Before adding a new system or helper, answer these questions:

1. What resource/state/registry is the authoritative owner of the fact?
2. Does an existing `SystemParam` already expose the required context?
3. Am I repeating a real invariant/transformation, or only code that looks similar?
4. Is there already a UI, queue, lifecycle, rendering, targeting, or budget primitive for this behavior?
5. Does this system need to run when world interaction is unavailable?
6. Can the update be change-driven instead of every frame?
7. Does the system signature remain comfortably below Bevy's parameter and tuple limits, with architectural review around ten meaningful dependencies?
8. Am I preserving HSI internally and converting only at a rendering/I/O boundary?
9. Is immutable content metadata being derived once by its owner rather than rediscovered in a hot path?
10. If this completes an update block, has `VERSION` been bumped appropriately?

When in doubt, prefer one authoritative owner plus small reusable primitives over mirrored state, copied systems, or one oversized context.
