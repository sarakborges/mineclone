# Chisel — runtime-only first delivery (2026-09-17)

Branch: `feature/chisel-microblocking`, based originally on `develop` commit `f06c68c39563b07095f383b060639463b0aac659`. Draft review/CI: [PR #12](https://github.com/sarakborges/mineclone/pull/12). Nothing merged into `develop` by this branch. Root `VERSION` remains `0.20.3`; `Cargo.toml` version `0.10.16` is independent. Do not bump until functional code and QA are complete. This document supersedes earlier two-level and 64³ prototypes.

## User-agreed mechanics

| Name in English | Divisions per axis | Cells on one face | Maximum volumetric cells |
| --- | ---: | ---: | ---: |
| Full | 1 | 1 | 1 |
| Thick | 2 | 4 | 8 |
| Thin | 4 | 16 | 64 |
| Extra Thin | 8 | 64 | 512 |

R cycles Full → Thick → Thin → Extra Thin → Full while Chisel is selected and the world interaction UI is available. Left mouse removes, right mouse places. No scroll wheel or add/remove mode. Extra Thin can make a panel one eighth of a full voxel thick. Every remaining or added microcell **inherits the macro parent block's ID, texture, orientation and visual properties**; this is shape editing, not multi-material voxel painting. A hollow log is a target use case: the hole is geometrically real and the player's AABB can pass through if there is space.

## Implemented in branch — source changes, not proof of runtime QA

- `src/voxel/microblock.rs`: 8×8×8 occupancy as eight `u64` layers, default-full macro cells have no mask. Edits snap to current precision and no-ops do not create mutations. Negative coordinates use Euclidean division. Macro material remains the canonical source. Temporary parents created in empty space carry an in-memory marker.
- `src/voxel/raycast.rs`, `src/voxel/collision.rs`: subvoxel ray traversal and AABB occupancy for sculpture holes; existing plain blocks take a fast path in collision.
- `src/voxel/mesh.rs`, `src/voxel/mesh/micro_mesh.rs`: greedy contiguous rectangle merging; culls adjacent microfaces and handles partial boundaries with normal blocks. Microcells share the existing chunk material buffers rather than becoming individual Bevy entities. Original macro textures are used with continuous subregion UVs.
- `src/tools/chisel.rs`: four-stage R input, left/right `ToolUse` pipeline, no independent microblock material, player placement intersection check, macro mutation remesh/lighting invalidation and transient-parent cleanup.
- `data/tools/chisel.json` and new `assets/textures/tools/chisel.png`: inventory definition and icon. `src/targeting/highlight.rs` previews an exact cut-size volume. `src/hud/chisel.rs` displays precision and controls; localization catalogs updated in English, Brazilian Portuguese and Spanish.
- In-session archive retains masks because `ArchivedChunk` holds raw `SecondaryProperties`. `SecondaryProperties::iter` hides the private shape from player-facing properties and disk serialization. `DiskChunk::from_chunk` omits Chisel-created temporary parents so they cannot revive as full blocks. Original macroblocks are saved without their session-only shape. **No microblock save/load is implemented or promised.**

## Current verification and blockers

Draft PR #12 triggered CI. First run `35256318279` passed localization but failed Clippy compilation because Bevy 0.19 has `TextLayout::justify`, not `new_with_justify`. Fixed in `dd33cfba697e9482dc0c46cf2b9cd85c547cc315`; further commits followed. Obtain latest PR CI result before declaring compile success. No `cargo test` was added/executed. Windows runtime QA and frame-time benchmarks were not performed in this environment.

**Known risks / unfinished QA:** macro-level lighting, fluid occupancy and some `VoxelWorld::is_solid` clients do not yet distinguish every cavity; test natural tree trunks, full-empty and restored states, adjoining carved blocks/chunk borders, negative coordinates, transparent surfaces, mesh winding/UV, session archive and save/restart baseline. The `SecondaryProperties` interner currently interns each distinct mask text (can retain memory across many edits); replace this provisional encoding with dedicated bounded per-chunk shape storage after runtime correctness is established. Run Clippy `--all-targets --all-features -- -D warnings`, cargo check and localization audit in CI; no `cargo test` without authorization.

## Next action

Resolve all CI compilation/lint issues, inspect current PR head check status, manually QA editing/movement/rendering in game on Windows, improve macro-level occupancy and resource lifetime as needed, then update this file and `HANDOFF.md` with actual outcome and checkpoint SHA. Do not merge the draft PR or bump `VERSION` while QA is pending.
