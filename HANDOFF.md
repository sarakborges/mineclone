# HANDOFF — Asteria / Mineclone

Current work: **World Systems Rebuild**

Authoritative architecture/phase contract:
- `docs/design/world-systems-rebuild.md`

Live implementation status:
- `docs/design/world-systems-rebuild-status.md`

Normative repository rules:
- `AGENTS.md`
- `ARCHITECTURE.md`
- `ENGINEERING_PRACTICES.md`

## Current implementation branch

`world-systems-rebuild`

The code baseline immediately before this documentation refresh was:

`a4b62bebeb22d7a4fe63558a0a939a083d01f54c` — `Author terrain profiles for all surface biomes`

Rust validation run `37249243969` completed successfully for that baseline.

## Current phase

**Phase 4 — Terrain: in progress.**

Do not restart Phase 3 unless Phase 4 exposes a real defect in the biome-layout contract.

## Completed work

### Phase 0 — Impact audit

External consumers and required capabilities are documented in the rebuild plan, including Structures/connectors, `/locate`, `/warp`, spawn/respawn, portals/dimension travel, streaming, persistence, loading, visuals and diagnostics.

### Phase 1 — Cleanup

The old world-generation stack was removed instead of being kept behind compatibility adapters.

Removed/retired ownership includes the legacy biome field/generation path, old generation regions/schedulers, obsolete loading/setup orchestration, old initial-presentation generation scheduling, old worldgen-specific persistence metadata, and temporary biome/tint bridges created during cleanup.

Preserved runtime infrastructure includes voxel/chunk state, rendering, remeshing, lighting, dynamic fluids, generic Structure authoring primitives, gameplay, inventory, crafting, storage and entities.

### Phase 2 — Generation foundation

The new generator foundation under `src/world/generator/` owns:

- immutable generation snapshot/state;
- deterministic named entropy domains;
- generation-owned world coordinate/request types;
- direct far-coordinate access;
- scalar and bounded query primitives;
- world-coordinate determinism independent of query/generation order;
- runtime lifecycle of the immutable `WorldGenerator` resource.

No semantic generation region is part of the world contract. `VoxelChunk` remains a materialization/runtime unit only.

### Phase 3 — Biome Layout

Implemented owners:

- `src/world/generator/biome.rs`
- `src/world/generator/biome_map.rs`
- `src/content/biome.rs`

Current surface biome contract includes:

- `surfaceLayout.weight`;
- `surfaceLayout.regionSize.min/max`;
- `surfaceLayout.cannotBorder`;
- one authoritative primary biome per X/Z position;
- normalized variable biome influences;
- scalar and bounded/grid surface queries;
- biome search;
- deterministic compatibility filtering and formation geometry.

`volume_biome_at` currently returns no override because no volume-biome content/field is authored yet. `effective_biome_at` therefore falls back to surface ownership. This is consistent with the design contract that the initial world may contain zero volume biomes.

The authoritative biome-map viewer is `src/world/generator/biome_map.rs`. It consumes the same `BiomeQueries` used by generation and supports PNG output, JSON legend, stable colors, center/scale/resolution, boundary overlay and influence blending. A duplicate map implementation was removed rather than maintained in parallel.

## Active Phase 4 state

Implemented so far:

- `src/world/generator/terrain.rs` owns `TerrainField` / `TerrainQueries`;
- `base_surface_at(x, z)` provides the continuous 2D base surface;
- `surface_at(x, z)` currently resolves the top of the base-solid terrain;
- `density_at(x, y, z)` currently resolves the base-solid density crossing;
- bounded terrain grid sampling reuses authoritative biome grid samples;
- biome influence weights blend terrain profiles continuously;
- terrain noise is world-coordinate anchored through deterministic generation domains;
- every current surface biome has an authored `surfaceTerrain` profile.

Current `surfaceTerrain` authoring fields:

- `baseHeightOffset`;
- `macroAmplitude`;
- `macroScale`;
- `detailAmplitude`;
- `detailScale`.

## Next concrete implementation

Continue Phase 4 by extending the **same** `TerrainField` into the authoritative final 3D terrain field.

Pending terrain work includes the true 3D contributions required by the plan, such as caves/carving, overhangs, floating formations, additive masses and future volume-biome terrain effects.

Effective surface/column queries must account for those 3D contributions without brute-force scanning the complete world height.

Do not create another terrain owner, another biome resolver, or another map implementation.

After Phase 4 is complete, proceed to Phase 5: surface/material composition and generated natural fluids.

## Validation rules that must not be forgotten

Root `AGENTS.md` is authoritative.

- **Do not run or add `cargo test` unless the user explicitly requests that command for the current task.**
- Default Rust gate is Clippy with `-D warnings`, `cargo check --locked`, and applicable content/localization/GLB audits.
- Do not claim a delivered implementation is complete until CI for the delivered SHA has completed successfully.
- Avoid broad repository walks/searches; use targeted reads.
- New features are forward-only; do not add compatibility shims unless explicitly requested.

## Non-negotiable worldgen rules

- No legacy worldgen resurrection.
- No semantic generation regions.
- No `land biome` abstraction.
- Ocean is a normal biome identity.
- No dedicated hydrology subsystem.
- Rivers are generic authored connected Structures/connectors.
- Queries must not materialize chunks merely to discover generated-world facts.
- Scalar/batch and generation-order behavior must remain semantically equivalent.
- One authoritative owner per generated fact.
