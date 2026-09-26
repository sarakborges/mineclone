# Automation refactor handoff — 2026-09-20 05:30

## Baseline

- Branch: `develop`.
- Starting HEAD: `2e5901726bd0cf997f70bd6fc4f9d6c411766f47` (`docs: unblock selected-slot persistence refactor`).
- Previous checkpoint: `docs/handoffs/automation-refactor-2026-09-20-0430.md`.
- Repository identity is now confirmed as `sarakborges/mineclone` through the installed GitHub connection.

## Work completed in this pass

Re-read the current `develop` ref and all three complete source blobs involved in selected-hotbar-slot persistence. No concurrent source changes were present: the blobs are still `8e5f776c6189b2d409c27e62f5fb3a8451f060b7` (`save_catalog.rs`), `4bb510edf9b2bf47fa36754ce5d107347d038bb4` (`save_session.rs`), and `1d091ff16e957a8c8477d6bdeee068f4621ecf92` (`world_selection.rs`).

The implementation plan from the prior handoff was revalidated against the complete source rather than excerpts. In particular, there are three save-capture paths that must stay consistent: `WorldSession::persist`, `WorldSaveContext::capture_owned`, and `WorldSaveContext::capture`. The selected slot must be copied into both `SavedWorldState` baselines and both `SnapshotSource` constructions, otherwise slot-only changes can either fail to dirty the autosave or be lost depending on save path.

No source patch was published in this pass. The available GitHub write primitives replace complete files or create complete blobs; although complete blob reads work, this execution environment does not expose a patch/edit primitive or a way to materialize those connector blobs into the local filesystem. Publishing three sequential whole-file replacements would transiently leave `develop` uncompilable, which is inappropriate for this cross-file schema change. The Git-tree primitives can publish atomically once the transformed complete contents are available to the write call.

## Exact coherent patch still required

### `src/world/save_catalog.rs`

1. In `WorldSnapshot`, immediately after `inventory`, add:
   `#[serde(default)] pub(crate) selected_hotbar_slot: usize,`
2. In `SnapshotSource`, add `pub(crate) selected_hotbar_slot: usize` next to `inventory`.
3. In `WorldSnapshot::capture`, copy `selected_hotbar_slot: source.selected_hotbar_slot`.
4. In `validate_playable`, after the inventory-length check, reject `snapshot.selected_hotbar_slot >= INVENTORY_SLOT_COUNT` with invalid-data error. This keeps old saves compatible because serde defaults the new field to slot 0.

### `src/world/save_session.rs`

1. Add `selected_hotbar_slot: usize` to `SavedWorldState` next to `inventory`.
2. In the baseline derived from `captured` inside `WorldSession::persist`, copy `captured.selected_hotbar_slot`.
3. In `saved_state`, copy `self.inventory.selected()` so a slot-only change participates in `PartialEq` dirty detection.
4. In `capture_owned`, capture `let selected_hotbar_slot = self.inventory.selected();`, pass it to `SnapshotSource`, and copy it into the resulting `SavedWorldState`.
5. In `capture`, pass `selected_hotbar_slot: self.inventory.selected()` to `SnapshotSource`.

### `src/screens/world_selection.rs`

Replace the load-time `restore_items(...)` call with `restore_items_and_selection(&snapshot.inventory, snapshot.selected_hotbar_slot, &context.content.blocks, &context.content.tools)`. Keep the existing error handling. The helper is already atomic and validates the selected index before mutating the live hotbar.

## Verification required after the patch

- Confirm `cargo fmt --check` in CI.
- Confirm Clippy with warnings denied.
- Confirm tests and save-fixture/backward-compatibility checks.
- Specifically cover an old snapshot with no selected-slot field (must restore 0), a selected index at the last valid inventory slot, and an out-of-range selected index (must reject the save before live inventory mutation).
- Confirm a slot-only selection change makes autosave dirty.
- Do not claim local Windows QA unless it was actually run.

## Lifecycle invariant

Do not restore the old unconditional `OnEnter(Gameplay)` reset. Session teardown already reinstalls `PlayerHotbar::default()` for a new world; loaded worlds need the persisted selected slot to survive entry into Gameplay.
