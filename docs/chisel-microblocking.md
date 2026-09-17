# Chisel — runtime-only first delivery (2026-09-17)

Branch: `feature/chisel-microblocking`, created from `develop` commit `f06c68c39563b07095f383b060639463b0aac659`. [Draft PR #12](https://github.com/sarakborges/mineclone/pull/12). No merge into `develop`. Root `VERSION` remains `0.20.3`; `Cargo.toml` has an independent version. Do not bump until functional QA is complete. This specification supersedes the former two-resolution, four-mode and 64³ prototypes.

## Agreed mechanics and controls

The Chisel has **three** resolutions, with no Full mode: whole-block removal or placement belongs to normal block tools.

| Chisel name | Divisions on each axis | Cells per face | Maximum cells in macroblock |
| --- | ---: | ---: | ---: |
| Thick | 2 | 2×2 | 8 |
| Thin | 4 | 4×4 | 64 |
| Extra Thin | 8 | 8×8 | 512 |

`R` cycles Thick → Thin → Extra Thin → Thick only while Chisel is selected and world interaction is available. Left mouse removes, right mouse places. No scroll, middle button, Full mode, or add/remove toggle. Extra Thin allows 1/8-thick trapdoors. Every microcell inherits its macroblock's ID, texture, orientation and visual properties: shape editing, not multi-material painting. Hollow logs have real carved-out space.

**Eligibility is explicit and safe by default:** `BlockDefinition.tags` is a validated string list. The `fragmentable` tag opts a block into Chisel. Currently `bassalt`, `dirt`, `grass`, `oak_log`, `sand`, `stone` are opted in. `oak_leaf`, `lamp` and `glass` are deliberately untagged and cannot be cut or used as a source/destination for Chisel placement. A future block without the tag is also ineligible by default. The gameplay edit handler validates both the hit macroblock and any existing destination macroblock; the targeting previews follow the same gating. This is data-driven, not an ID blacklist or a category-based inference.

**Tooltip placement:** Chisel information reuses the existing Brush `ActionHint` under the crosshair via `src/hud/crosshair.rs`, never a dedicated label above the hotbar. It respects `HudSettings.display_tooltips`, hides on creature targets or unsupported blocks and refreshes when R changes precision. Removed the old `src/hud/chisel.rs` HUD plugin and file.

## Code integration

- `src/voxel/microblock.rs`: eight `u64` occupancy layers (8³) for modified macroblocks; regular blocks require no mask. Snapping, no-op suppression and negative Euclidean coordinates. Temporary parent blocks placed in an otherwise empty voxel carry a session marker.
- `src/voxel/raycast.rs` and `src/voxel/collision.rs`: subvoxel targeting and AABB occupancy for carved openings; regular blocks retain the fast collision path.
- `src/voxel/mesh.rs` and `src/voxel/mesh/micro_mesh.rs`: greedy contiguous face merging, local and macro neighbor occlusion, shared chunk material buffers, original macro texture/orientation and subregion UVs. No Bevy entity per microcell.
- `src/tools/chisel.rs`: `ToolUse` left/right, R cycle, `fragmentable` enforcement on hit and destination, material inheritance, player intersection check, chunk remesh/lighting invalidation and removal of empty temporary parents. No Full-level whole-block break.
- `data/tools/chisel.json` and `assets/textures/tools/chisel.png`: definition/icon. `src/targeting/highlight.rs`: white cut-size preview and green placement preview; forbidden hit and occupied forbidden destination suppress previews. `src/hud/crosshair.rs`: shared Brush location for contextual precision/actions, translated EN/PT-BR/ES.
- Runtime chunk archives retain masks via raw `SecondaryProperties`; disk snapshots save original macroblocks without Chisel shape and omit temporary parents, so they never reload as full blocks. **Micro geometry intentionally does not persist across game restart.**

## Verification and limitations

[CI run 35257019737](https://github.com/sarakborges/mineclone/actions/runs/35257019737) passed language audit, strict Clippy (`--all-targets --all-features -- -D warnings`) and cargo check for earlier source commit `e7f4911829e07b988d1a3dda699fba54956cde4f`. The preview and new tag/three-level/tooltip commits are later and require fresh CI; do **not** describe them as already validated. No `cargo test` was added or executed. Windows gameplay QA and performance benchmarks remain unperformed.

Macro-resolution lighting, fluid occupancy and other `VoxelWorld::is_solid` callers may still treat some carved cavities as solid. The temporary `SecondaryProperties` interner retains distinct masks across session edits; a bounded per-chunk shape overlay remains necessary for production memory behavior. Autosave revision may advance despite runtime-only shape changes. Gameplay QA required for eligible/ineligible materials, preview consistency, camera targeting, rotations/UV, all three levels, hollow trunks, borders and negative coords, collision, chunk archive and restart fallback. Keep PR draft, `develop` unchanged and `VERSION` unchanged until QA/closure. Do not merge without explicit user instruction.
