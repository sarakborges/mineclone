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

Use schedule sets to express ordering between domains. Prefer named sets over chains that only exist to compensate for hidden side effects.

## 5. UI design system ownership

Reusable UI behavior and appearance belong under `src/ui`.

- Screens and HUD modules resolve domain data and assemble layouts; they should not clone interaction mechanics or control styling.
- View-only state such as search focus, filter text, selected presentation categories, scroll positions, and UI dirtiness belongs to the owning HUD/screen module rather than gameplay/player domains.
- Action buttons use the canonical `ui::button::button(...)` primitive and `ButtonVariant`; screens must not create parallel button constructors for the same interaction model.
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

Surface-carver tunnels are subordinate to cave connectivity: a surface tunnel is generated only when its carved volume intersects the anchored cave connector graph. Disconnected surface tunnels/dead ends must be rejected before density rasterization, and structure-support sampling must use the same connectivity rule.

Spawn-column dryness must use the same physically supported hydrology as terrain generation. When actual terrain surface height is available, bootstrap/spawn selection must use `supported_water_at(...)` rather than unfiltered `water_at(...)`, so unsupported lake/ocean candidates cannot make genuinely dry terrain impossible to select.

A user-selected Spawn Biome is not a request to search the seed for a distant natural occurrence. It is a deterministic initial-region override owned by `BiomeField`: the canonical 9×9 bootstrap chunks around the default spawn are forced to the selected surface biome, with the same override feeding terrain sampling, biome identity and continentalness/hydrology so Coast/Ocean cannot supersede it. Only dry-column placement is searched inside that already-forced core. The selected spawn biome is persisted in world snapshots and restored on load so regenerated/queried initial terrain keeps the same identity across sessions.

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
