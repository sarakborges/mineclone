# Automation refactor checkpoint — 2026-09-20 00:30 BRT

## Context read

Continued from `docs/handoffs/automation-refactor-2026-09-20-0000.md` on `develop`. The active persistence roadmap remains selected-hotbar-slot persistence, then player/camera look rotation, clock-only dirty-state design, then incremental chunk/region persistence.

## Verification completed in this run

GitHub Actions run `35417348061` (`Rust validation`) for commit `9a0a59b9dfa423d8e0c6e95ce543949c820dc10b` completed successfully. The preparatory `PlayerHotbar::restore_items_and_selection(...)` change is therefore green under canonical CI.

No `cargo test`, `cargo run`, Windows QA, PNG/GLB changes, or version bump were performed.

## Wiring audit completed

Re-read the complete selected-slot path before changing the disk schema:

- `src/world/save_catalog.rs`: `WorldSnapshot` currently stores `inventory` but not selection; `SnapshotSource` likewise lacks selection; `validate_playable()` validates inventory length/items but not a selected index; `WorldSnapshot::capture()` is the schema capture point.
- `src/world/save_session.rs`: `SavedWorldState` lacks selection, so a slot-only change cannot trigger autosave. Both `saved_state()` and `capture_owned()` must include it, and the synchronous `persist()` baseline must derive it from the exact captured snapshot. Both snapshot capture paths already own `Res<PlayerHotbar>`, so `selected_slot()` is available without another query.
- `src/screens/world_selection.rs`: accepted loads still call `restore_items(...)`; this must become `restore_items_and_selection(..., snapshot.selected_hotbar_slot, ...)`.
- `src/player/hotbar.rs`: `OnEnter(GameState::Gameplay)` still calls `reset_hotbar_selection`, which would overwrite a restored selection. Simply deleting it is insufficient because `PlayerHotbar` is a persistent resource and a subsequent fresh world could inherit the previous world's selected slot. Reset must move to an explicit fresh/new-world path or be gated by load mode.

## Implementation status

No schema mutation was committed in this run. The GitHub connector available to this automation replaces complete files rather than applying hunks, and the coherent change spans the 35-KiB `save_catalog.rs`, 15-KiB `save_session.rs`, 29-KiB `world_selection.rs`, and hotbar lifecycle behavior. Committing only one side would deliberately leave `develop` uncompilable or produce incorrect fresh-world selection behavior. This checkpoint records the exact atomic patch boundary instead of publishing a broken intermediate state.

## Exact next implementation

1. In `save_catalog.rs`, import `HOTBAR_SLOT_COUNT` alongside `INVENTORY_SLOT_COUNT`.
2. Add `#[serde(default)] pub(crate) selected_hotbar_slot: usize` immediately after `inventory` in `WorldSnapshot`; old saves therefore load slot 0.
3. Add `pub(crate) selected_hotbar_slot: usize` to `SnapshotSource`.
4. Reject `snapshot.selected_hotbar_slot >= HOTBAR_SLOT_COUNT` in `validate_playable()` with `invalid selected hotbar slot`.
5. Reject `source.selected_hotbar_slot >= HOTBAR_SLOT_COUNT` during `WorldSnapshot::capture()` before expensive chunk capture, then copy it into the snapshot.
6. In `save_session.rs`, add `selected_hotbar_slot: usize` to `SavedWorldState`; populate it from `captured.selected_hotbar_slot` in synchronous `persist()`, from `self.inventory.selected_slot()` in `saved_state()`, and from the captured/current hotbar in `capture_owned()`.
7. Pass `selected_hotbar_slot: self.inventory.selected_slot()` in both `SnapshotSource` construction sites.
8. In `world_selection.rs`, replace `restore_items(...)` with `restore_items_and_selection(&snapshot.inventory, snapshot.selected_hotbar_slot, ...)`.
9. Fix the gameplay-entry reset without regressing fresh worlds. Preferred direction: reset selection when beginning a genuinely new world, not on every `OnEnter(Gameplay)`. Confirm the existing `WorldLoadMode::{New,Load}` lifecycle in `world/mod.rs`/setup before choosing the smallest gate.
10. Observe canonical CI for the atomic wiring commit. Do not run `cargo test` without explicit authorization.
11. Once green, continue immediately with player/camera look persistence.
