# Automation refactor handoff — 2026-09-20 06:30

## Baseline

- Branch: `develop`.
- Starting HEAD: `e853f093bd0a28f9c424a8db2534cc1757197c3c` (`docs: checkpoint selected-slot implementation`).
- Previous checkpoint: `docs/handoffs/automation-refactor-2026-09-20-0530.md`.

## Work completed in this pass

Re-read the complete source blobs for `save_catalog.rs`, `save_session.rs`, `world_selection.rs`, plus the current `src/player/hotbar.rs` implementation. No concurrent source change was present at the start of the pass.

This audit found and corrected an important mistake in the previous implementation plan: `selected_hotbar_slot` is a HOTBAR index, not an inventory index. `PlayerHotbar::restore_items_and_selection` already rejects `selected_slot >= HOTBAR_SLOT_COUNT`; therefore save validation must use the same bound. Validating against `INVENTORY_SLOT_COUNT` would incorrectly admit indices 9..35 into a supposedly playable snapshot, only for activation to fail later.

No source patch was published in this pass. The connector exposes complete blob reads and whole-file/blob writes but no patch primitive. The cross-file schema change still needs transformed complete contents for three files before publishing one atomic Git tree. Sequential writes directly to `develop` remain inappropriate because intermediate commits would not compile.

## Exact coherent patch still required

### `src/world/save_catalog.rs`

1. Extend the hotbar import to include `HOTBAR_SLOT_COUNT` (keep `INVENTORY_SLOT_COUNT` for inventory-length validation).
2. In `WorldSnapshot`, immediately after `inventory`, add:
   `#[serde(default)] pub(crate) selected_hotbar_slot: usize,`
3. In `SnapshotSource`, add `pub(crate) selected_hotbar_slot: usize` next to `inventory`.
4. In `WorldSnapshot::capture`, copy `selected_hotbar_slot: source.selected_hotbar_slot`.
5. In `validate_playable`, after the inventory-length check, reject `snapshot.selected_hotbar_slot >= HOTBAR_SLOT_COUNT` with invalid-data error. Old saves remain compatible because serde defaults the new field to slot 0.

### `src/world/save_session.rs`

1. Add `selected_hotbar_slot: usize` to `SavedWorldState` next to `inventory`.
2. In the baseline derived from `captured` inside `WorldSession::persist`, copy `captured.selected_hotbar_slot`.
3. In `saved_state`, copy `self.inventory.selected_slot()` so a slot-only change participates in `PartialEq` dirty detection. Note the actual accessor is `selected_slot()`, not `selected()`.
4. In `capture_owned`, capture `let selected_hotbar_slot = self.inventory.selected_slot();`, pass it to `SnapshotSource`, and copy it into the resulting `SavedWorldState`.
5. In `capture`, pass `selected_hotbar_slot: self.inventory.selected_slot()` to `SnapshotSource`.

### `src/screens/world_selection.rs`

Replace the load-time `restore_items(...)` call with `restore_items_and_selection(&snapshot.inventory, snapshot.selected_hotbar_slot, &context.content.blocks, &context.content.tools)`. Keep existing error handling. The helper validates both inventory length/content and the hotbar index before mutating live inventory, so activation remains atomic.

## Verification required after the patch

- `cargo fmt --check`.
- Clippy with warnings denied.
- Tests/save-fixture backward compatibility.
- Old snapshot without the field restores slot 0.
- Slot 8 is accepted and restored; slot 9 is rejected before live inventory mutation.
- A slot-only selection change makes autosave dirty.
- Preserve the lifecycle invariant: do not restore the old unconditional `OnEnter(Gameplay)` hotbar reset.
- Do not claim local Windows QA unless actually run.

## Important correction carried forward

Previous handoffs that say to validate `selected_hotbar_slot` against `INVENTORY_SLOT_COUNT` or use `self.inventory.selected()` are stale. The correct bound/accessor are `HOTBAR_SLOT_COUNT` and `selected_slot()` respectively, as verified against the current `PlayerHotbar` implementation.
