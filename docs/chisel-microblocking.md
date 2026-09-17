# Chisel microblocking — implementation checkpoint

Branch: `feature/chisel-microblocking`, created from `develop` at `f06c68c39563b07095f383b060639463b0aac659` on 2026-09-17.

Version authority: root `VERSION` was `0.20.3` at branch creation. `Cargo.toml` independently declared `0.10.16`; do not confuse these or synchronize them without a separate decision. Do not bump `VERSION` before finishing a functional block, per `HANDOFF.md`.

## Agreed controls

- Left mouse button: remove the selected microblock.
- Right mouse button: place the selected microblock.
- `R`: toggle 1/8 (2x2x2 subdivisions of a full block) and 1/64 (4x4x4 subdivisions).
- No scroll wheel, middle button, V/B mode switching, or add/remove mode state. Respect gameplay input/UI focus and support remapping through the eventual input configuration.
- Both buttons act immediately; only precision is stateful. Highlight the exact region targeted before an edit.

## Current code checkpoint

`src/voxel/microblock.rs` introduces the first **isolated** storage primitive. One 4x4x4 fine grid accommodates both modes. A coarse click updates one aligned 2x2x2 region, a fine click changes one cell. Edits that do not change contents return `false`; out-of-bounds coordinates are rejected. Storage starts as a uniform cell, is allocated on first meaningful partial edit and becomes uniform again when all 64 cells match. It reuses `VoxelCell` for material/properties rather than inventing another material identity.

**Not yet playable:** this new source file is deliberately not added to `src/voxel/mod.rs`, because there is no consumer yet. Do not present it as compiled, tested, functional gameplay or a complete feature. It is a foundation for the next implementation steps, and no version bump is due at this checkpoint.

## Integration sequence / correctness gates

1. Bind per-voxel microblock storage to the existing chunk lifecycle, including archived/unloaded chunks, without allocating 64 cells for ordinary voxels. Register the module when it has a real consumer; keep builds free of dead-code warnings and avoid lint suppressions.
2. Integrate the Chisel tool definition and event path: the existing `ToolUse` already distinguishes left and right clicks (`src/targeting/interaction.rs`); add `R` only while the Chisel is equipped and world interaction is available. Determine selected placeable material from the existing inventory conventions, not a fictitious infinite source.
3. Implement hit selection and boundary-safe placement/removal: expose the actual fine-grid coordinates and face normal, and handle adjacent parent blocks, empty targets and negative world coordinates consistently.
4. Integrate mesh generation with face culling across microblock and regular block boundaries and dirty neighbor updates. Do not render each microblock as a standalone Bevy entity.
5. Update collision and occupancy queries; connect the correct lighting invalidation. Avoid turning partially excavated voxels into whole-block air or whole-block colliders.
6. Extend save/archive format and backward compatibility so edits survive unload, restart, fallback snapshots and recovery. Preserve canonical save revision semantics and never overwrite user edits with procedural generation.
7. Make the target preview and current precision visible in the HUD, localize any new text in English, PT-BR and Spanish, and perform static checks plus game QA as permitted by `HANDOFF.md`.

No `cargo test` was added or run. Neither `cargo check`, Clippy nor Windows gameplay QA has been executed for this branch at this checkpoint. Do not claim otherwise.
