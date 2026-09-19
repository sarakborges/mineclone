# Automation refactor checkpoint — 2026-09-20 02:30 BRT

## Context read

Continued from `docs/handoffs/automation-refactor-2026-09-20-0130.md` on `develop`. The active persistence roadmap remains selected-hotbar-slot persistence, then player/camera look rotation, clock-only dirty-state design, then incremental chunk/region persistence.

## Verification completed in this run

Canonical Rust validation run `35421561114` for commit `3f69c19a78393a4922fc7a7a7427bf45e9545970` completed successfully. The hotbar lifecycle cleanup therefore has a green CI baseline and the selected-slot schema work can proceed.

The current branch was re-read directly. `src/player/hotbar.rs` already exposes the atomic `restore_items_and_selection(...)` path and validates `selected_slot < HOTBAR_SLOT_COUNT`. `src/world/save_catalog.rs` still persists only `inventory`, `src/world/save_session.rs` still excludes selection from `SavedWorldState` and both snapshot capture paths, and `src/screens/world_selection.rs` still calls `restore_items(...)`. Therefore the remaining gap is exactly the coherent schema/autosave/load wiring described below; no hidden lifecycle reset remains.

## Remaining selected-slot implementation

1. In `src/world/save_catalog.rs`, import `HOTBAR_SLOT_COUNT` alongside `INVENTORY_SLOT_COUNT`.
2. Add `#[serde(default)] pub(crate) selected_hotbar_slot: usize` immediately after `inventory` in `WorldSnapshot`; old saves therefore restore slot 0.
3. Add `pub(crate) selected_hotbar_slot: usize` to `SnapshotSource`.
4. Reject `snapshot.selected_hotbar_slot >= HOTBAR_SLOT_COUNT` in `validate_playable()` with `invalid selected hotbar slot`.
5. Reject `source.selected_hotbar_slot >= HOTBAR_SLOT_COUNT` during `WorldSnapshot::capture()` before expensive fluid/chunk capture, then copy it into the snapshot.
6. In `src/world/save_session.rs`, add `selected_hotbar_slot: usize` to `SavedWorldState`; populate it from `captured.selected_hotbar_slot` in synchronous `persist()`, from `self.inventory.selected_slot()` in `saved_state()`, and from the captured/current hotbar in `capture_owned()`.
7. Pass `selected_hotbar_slot: self.inventory.selected_slot()` in both `SnapshotSource` construction sites.
8. In `src/screens/world_selection.rs`, replace `restore_items(...)` with `restore_items_and_selection(&snapshot.inventory, snapshot.selected_hotbar_slot, ...)`.
9. Publish those three source-file changes as one coherent commit and observe canonical CI before starting player/camera look persistence.

## Connector constraint

The available GitHub write actions replace complete files rather than applying hunks. This change spans source files of roughly 35 KiB, 15 KiB, and 29 KiB. Publishing them sequentially would temporarily leave `develop` with a schema that does not compile; this run did not intentionally create that broken intermediate state. The next executor should prefer an atomic Git tree/commit write if available, or otherwise perform the complete-file replacements back-to-back and immediately verify the final head.

No `cargo test`, `cargo run`, Windows QA, PNG/GLB changes, or version bump were performed.
