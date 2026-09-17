# Chisel — first runtime-only delivery (2026-09-17)

Branch: `feature/chisel-microblocking`, created from `develop` at `f06c68c39563b07095f383b060639463b0aac659`. [Draft PR #12](https://github.com/sarakborges/mineclone/pull/12). No merge into `develop`. Root `VERSION` remains `0.20.3`, independent from `Cargo.toml` `0.10.16`. Do not bump before functional QA. This specification supersedes earlier two-resolution and 64³ prototypes.

## User-agreed mechanics

| English name | Subdivisions per axis | Face grid | Max microcells |
| --- | ---: | ---: | ---: |
| Full | 1 | 1×1 | 1 |
| Thick | 2 | 2×2 | 8 |
| Thin | 4 | 4×4 | 64 |
| Extra Thin | 8 | 8×8 | 512 |

`R`: cycle Full → Thick → Thin → Extra Thin → Full while Chisel is equipped and UI/world interaction is available. Left mouse removes, right mouse places. No add/remove mode, scroll wheel or middle button. Extra Thin supports one-eighth-block-thick trapdoors. **Every piece inherits the macro block's ID, texture, orientation and visual properties.** The tool edits shape, not microcell material. A naturally textured hollow tree trunk with real empty center and appropriate collision is a reference scenario.

## Integrated source (not a substitute for gameplay QA)

- `src/voxel/microblock.rs`: 8×8×8 shape in eight `u64` masks. Normal macroblocks have no mask; cuts snap to precision and skip no-ops. Euclidean world coordinates handle negative axes. Temporary macro parents in empty space are marked transient.
- `src/voxel/raycast.rs`, `src/voxel/collision.rs`: 8-grid ray traversal and actual AABB subcell collision. A regular full cube keeps its simple collision path.
- `src/voxel/mesh.rs`, `src/voxel/mesh/micro_mesh.rs`: greedy face rectangles, neighboring-cell occlusion and partial regular/micro boundaries; all faces grouped in the normal chunk material buffers (no entity per piece). Original macroblock textures and orientation are used, with subregion UVs.
- `src/tools/chisel.rs`: existing `ToolUse` left/right, R precision cycle, material inheritance on right placement, player overlap guard, chunk remesh/lighting invalidation and removal of empty temporary parents.
- Tool definition `data/tools/chisel.json` and new icon `assets/textures/tools/chisel.png`. `src/targeting/highlight.rs` now has distinct exact-size **white removal** and **green placement** previews; both follow selected precision and hit face, with the placement ghost hidden when target chunk is unloaded or fluid blocks placement. `src/hud/chisel.rs` displays active precision/controls, translated in EN/PT-BR/ES.
- Runtime chunk archives retain masks via raw `SecondaryProperties`. `SecondaryProperties::iter` keeps the private mask out of public/disk properties. `DiskChunk::from_chunk` saves the original macro material without micro geometry and omits Chisel-created transient parents; they cannot reload as full cubes. **Micro shapes are intentionally not persisted across restart.**

## Verification

[CI run 35257019737](https://github.com/sarakborges/mineclone/actions/runs/35257019737) for source commit `e7f4911829e07b988d1a3dda699fba54956cde4f`: audit of three languages, strict Clippy (`cargo clippy --locked --all-targets --all-features -- -D warnings`) and `cargo check --locked` all **passed**. Earlier failures (`TextLayout::new_with_justify`, dead `block_id_at`) were fixed, without lint suppression. Additional preview code commit `b462fe9a6819df1a7ee504744f1d9c3aa7e5b86e` followed the green run and requires its own CI result. No `cargo test` added or run. Windows gameplay QA and benchmarks have not run.

## Known limitations and essential gameplay checks

Macro-resolution lighting, fluid occupancy and other callers of `VoxelWorld::is_solid` do not yet represent every opening; player collision and raycasts use the detailed shape. The `SecondaryProperties` token interner retains each distinct occupancy string over the session, so replace this provisional shape storage with a bounded per-chunk overlay before claiming production-grade memory behavior. Autosave revision can advance from session-only shape changes. No measured performance data, no QA for winding/UV, texture rotation, adjacent chunk edges, continuous hollow trunks, extreme coordinates, full-empty/restored state, transient temporary parents, streaming archive, fluid interaction or clean restart fallback.

Keep PR #12 **draft**, keep develop and VERSION unchanged, resolve latest preview commit CI, test actual editing/rendering/player collision in-game, and update this checkpoint plus root HANDOFF only with observed outcomes. Do not merge without explicit instruction.
