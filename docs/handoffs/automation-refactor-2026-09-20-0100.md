# Automation refactor checkpoint — 2026-09-20 01:00 BRT

## Context read

Continued from `docs/handoffs/automation-refactor-2026-09-20-0030.md` on `develop`. The active persistence roadmap remains selected-hotbar-slot persistence, then player/camera look rotation, clock-only dirty-state design, then incremental chunk/region persistence.

## Implemented in this run

Commit `e810f8b8716f27c8a69e4d88de6b6b9a0229e3f8` removes the unconditional `OnEnter(GameState::Gameplay)` hotbar-selection reset from `PlayerHotbarPlugin`.

This closes the lifecycle blocker identified in the previous handoff: once a loaded snapshot restores a selected slot, entering Gameplay will no longer overwrite it with slot 0. Fresh-world behavior remains safe because `PlayerHotbar` defaults to slot 0 and `release_world_session()` replaces the persistent hotbar resource with `PlayerHotbar::default()` when returning to the starting screen. No schema mutation was included in this commit, so current saves still restore through the existing inventory-only path until the atomic persistence wiring lands.

## Verification

Re-read `src/world/mod.rs` before removing the reset and confirmed that world-session release explicitly installs `PlayerHotbar::default()`, preventing a subsequent fresh world from inheriting a previous world's selected slot through the normal world lifecycle.

Immediately after publication, no GitHub Actions workflow run was yet associated with commit `e810f8b8716f27c8a69e4d88de6b6b9a0229e3f8`; do not treat CI as green until a later run observes its result. No `cargo test`, `cargo run`, Windows QA, PNG/GLB changes, or version bump were performed.

## Remaining selected-slot implementation

1. In `save_catalog.rs`, import `HOTBAR_SLOT_COUNT` alongside `INVENTORY_SLOT_COUNT`.
2. Add `#[serde(default)] pub(crate) selected_hotbar_slot: usize` immediately after `inventory` in `WorldSnapshot`; old saves therefore load slot 0.
3. Add `pub(crate) selected_hotbar_slot: usize` to `SnapshotSource`.
4. Reject `snapshot.selected_hotbar_slot >= HOTBAR_SLOT_COUNT` in `validate_playable()` with `invalid selected hotbar slot`.
5. Reject `source.selected_hotbar_slot >= HOTBAR_SLOT_COUNT` during `WorldSnapshot::capture()` before expensive fluid/chunk capture, then copy it into the snapshot.
6. In `save_session.rs`, add `selected_hotbar_slot: usize` to `SavedWorldState`; populate it from `captured.selected_hotbar_slot` in synchronous `persist()`, from `self.inventory.selected_slot()` in `saved_state()`, and from the captured/current hotbar in `capture_owned()`.
7. Pass `selected_hotbar_slot: self.inventory.selected_slot()` in both `SnapshotSource` construction sites.
8. In `world_selection.rs`, replace `restore_items(...)` with `restore_items_and_selection(&snapshot.inventory, snapshot.selected_hotbar_slot, ...)`.
9. Observe canonical CI for the coherent schema/wiring commit. Do not run `cargo test` without explicit authorization.
10. Once green, continue immediately with player/camera look persistence.

## Connector constraint

The GitHub connector writes complete file contents rather than applying hunks. `save_catalog.rs` is large enough that the coherent cross-file schema change should still be published atomically rather than leaving `develop` with only part of the new field wired. This run therefore used the safe independent lifecycle commit first and left the exact schema boundary above for the next execution.
