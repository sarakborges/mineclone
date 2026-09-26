# Automation refactor handoff — 2026-09-20 07:30

## Baseline

- Branch: `develop`.
- Starting HEAD: `f06faa6688c13f1f0e94227458d5c3628fd6ab0b` (`docs: correct selected-slot validation checkpoint`).
- Previous checkpoint: `docs/handoffs/automation-refactor-2026-09-20-0630.md`.

## Work completed in this pass

Created staging branch `automation/selected-hotbar-slot` from the exact starting `develop` HEAD so the cross-file schema change can be assembled without ever leaving `develop` in a non-compiling intermediate state.

Implemented the complete `save_session.rs` portion of selected-hotbar-slot persistence on that staging branch in commit `f15ae78de8b84e5583b9f552e0af137b5bfdbdd1` (`refactor: capture selected hotbar slot in save session`). Specifically:

- `SavedWorldState` now includes `selected_hotbar_slot`, so a selection-only change participates in `PartialEq` dirty detection.
- synchronous `persist` derives the successful baseline from `captured.selected_hotbar_slot`, preserving the invariant that the baseline exactly matches the snapshot actually written;
- `saved_state` captures `PlayerHotbar::selected_slot()`;
- `capture_owned` captures the selected slot once, passes it to `SnapshotSource`, and copies the same value into its `SavedWorldState` baseline;
- `capture` passes `selected_hotbar_slot: self.inventory.selected_slot()` to `SnapshotSource`.

This staging commit intentionally does not compile by itself yet because `WorldSnapshot` and `SnapshotSource` on the branch do not have the new field until the `save_catalog.rs` portion is added. It was therefore NOT merged/moved onto `develop`.

## Remaining coherent patch

Continue on `automation/selected-hotbar-slot` (currently `f15ae78de8b84e5583b9f552e0af137b5bfdbdd1`) and finish the other two source files before publishing one atomic final tree/commit onto `develop`.

### `src/world/save_catalog.rs`

1. Extend `player::hotbar::INVENTORY_SLOT_COUNT` import to `{HOTBAR_SLOT_COUNT, INVENTORY_SLOT_COUNT}`.
2. In `WorldSnapshot`, immediately after `inventory`, add `#[serde(default)] pub(crate) selected_hotbar_slot: usize,`.
3. In `SnapshotSource`, add `pub(crate) selected_hotbar_slot: usize` next to `inventory`.
4. In `WorldSnapshot::capture`, copy `selected_hotbar_slot: source.selected_hotbar_slot`.
5. In `validate_playable`, immediately after inventory-length validation, reject `snapshot.selected_hotbar_slot >= HOTBAR_SLOT_COUNT` with invalid-data error. Do not use `INVENTORY_SLOT_COUNT` for this bound. Serde default keeps old saves compatible at slot 0.

### `src/screens/world_selection.rs`

Replace load-time `restore_items(...)` with `restore_items_and_selection(&snapshot.inventory, snapshot.selected_hotbar_slot, &context.content.blocks, &context.content.tools)`, preserving existing error handling. The helper already validates inventory and selection before mutating live inventory.

## Publication strategy

Do not merge the staging branch sequentially into `develop`. After both remaining files are complete, use the staging branch's final tree SHA to create a new commit whose parent is the then-current `develop` HEAD, but only if `develop` has not acquired conflicting source changes. Then fast-forward `develop` to that commit. This gives `develop` one coherent source commit even though the connector only supports whole-file writes.

If `develop` moved only because of this handoff commit, use that new handoff commit as the parent and the staging tree as the source tree, but ensure the new tree also retains this handoff file (build from current develop base or add the handoff blob when creating the final tree).

## Verification required after coherent publication

- CI / `cargo fmt --check` and Clippy with warnings denied.
- Tests/save-fixture backward compatibility.
- Old snapshot without the field restores slot 0.
- Slot 8 is accepted/restored; slot 9 is rejected before live inventory mutation.
- A slot-only selection change is dirty and gets autosaved.
- Preserve the lifecycle invariant: do not restore the old unconditional `OnEnter(Gameplay)` hotbar reset.
- Do not claim local Windows QA unless actually run.

## Important state

`develop` remains compiling-equivalent to the prior checkpoint; only this handoff is being added there. The source work lives exclusively on `automation/selected-hotbar-slot` until the schema/load changes are complete.
