# Chisel microblocking — implementation checkpoint

Branch: `feature/chisel-microblocking`, created from `develop` at `f06c68c39563b07095f383b060639463b0aac659` on 2026-09-17.

Version authority: root `VERSION` was `0.20.3` at branch creation. `Cargo.toml` independently declared `0.10.16`; do not confuse these or synchronize them without a separate decision. Do not bump `VERSION` before finishing a functional block, per `HANDOFF.md`.

## Final precision terminology and controls (2026-09-17)

The four **player-facing English names** and their exact meanings are:

| HUD label | Subdivisions per axis | Cells per face | Maximum cells per parent voxel |
| --- | --- | --- | --- |
| Full | 1 | 1 (1×1) | 1 |
| Thick | 2 | 4 (2×2) | 8 |
| Thin | 4 | 16 (4×4) | 64 |
| Extra Thin | 8 | 64 (8×8) | 512 |

`ExtraThin` is the Rust variant; **Extra Thin** is the player-facing label. Do **not** interpret 64 face cells as 64 subdivisions along each axis or as 262,144 cells per block. An 1/8-thick trapdoor is made using one layer of the 8 subdivisions available at Extra Thin. Previous suggestions such as `Whole/Half/Quarter/Eighth`, `Deep Level`, and displaying the levels simply as `1/4/16/64` are superseded as player-facing names.

- Left mouse button: remove the selected microblock.
- Right mouse button: place the selected microblock.
- `R`: cycle **Full → Thick → Thin → Extra Thin → Full** only when the Chisel is selected and world interaction is available.
- No scroll wheel, middle button, V/B mode switching, or add/remove mode state. Respect gameplay input/UI focus and eventual input remapping.
- Both buttons act immediately; only precision is stateful. Preview the exact region targeted before an edit.
- Player-facing names in other languages must be localized in English, PT-BR and Spanish when the HUD is connected, retaining these English names as the canonical design terminology.

## Current code checkpoint

`src/tools/chisel.rs` now declares the four `ChiselResolution` variants (`Full`, `Thick`, `Thin`, `ExtraThin`) and cycles them on `R` under the appropriate selected-item and interaction checks. It is registered through `ToolsPlugin`, but the Chisel item has intentionally **not** been made available in-game because its editing pipeline is incomplete.

`src/voxel/microblock.rs` remains the *earlier isolated prototype* with a **4×4×4 grid and only two resolutions**. It has deliberately not been registered in `src/voxel/mod.rs` and **does not implement the agreed four-level specification**. Its storage/edit logic must be redesigned or extended to support 8×8×8 with sparse/uniform region compaction before gameplay integration. Do not confuse the existence of the input-state code with a playable Chisel or imply that `Extra Thin` already edits voxels. Ordinary, unmodified voxels must remain compact.

## Integration sequence / correctness gates

1. Implement per-voxel four-level storage in the existing chunk lifecycle, including archived/unloaded chunks, without allocating 512 cells for ordinary voxels. Register the module once consumers exist; maintain zero dead-code warnings and do not use lint suppressions.
2. Integrate the Chisel item and `ToolUse` events: left removes, right places, `R` cycles only while selected and interaction is available. Determine selected placeable material from existing inventory conventions rather than inventing infinite material supply.
3. Implement accurate targeting and boundary-safe placement/removal at 1/2/4/8 subdivisions per axis, including negative coordinates, adjacent parent blocks, and empty targets.
4. Mesh the actual microgeometry, merge coplanar surfaces/occlude faces across ordinary and detailed blocks, invalidate neighbor meshes, and avoid one Bevy entity per cell. Benchmark realistic and worst-case patterns.
5. Update collision and occupancy; connect lighting invalidation. Partially excavated blocks must not behave as whole-block air or solid cubes.
6. Extend on-disk chunk representation and archived-chunk roundtrip with backward compatibility and correct save revisions; edits must survive unload, restart, snapshot fallback and recovery without procedural overwrites.
7. Add targeted-region preview and localized HUD labels for the four names; audit EN/PT-BR/ES and perform code checks and runtime QA per `HANDOFF.md`.

No `cargo test` was added or run. Neither `cargo check`, strict Clippy nor Windows gameplay QA has been executed for this branch at this checkpoint. No root version bump: functionality is incomplete.
