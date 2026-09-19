# Automation refactor handoff — 2026-09-20 04:30

## Baseline

- Branch: `develop`.
- Starting HEAD: `4ed83df3e86889753dcc35fdb412b4b7eb11ee10` (`docs: checkpoint selected-slot persistence work`).
- Previous checkpoint: `docs/handoffs/automation-refactor-2026-09-20-0330.md`.
- The selected-hotbar-slot persistence gap remains the active refactor item.

## Work completed in this pass

The previous connector-read blocker was resolved. Reading `src/world/save_catalog.rs` through the Git blob endpoint returns the complete 35,904-byte source, so a future pass can safely construct whole-file replacements instead of relying on truncated `fetch_file` output.

Revalidated the implementation surface against current `develop`:

1. `src/world/save_catalog.rs`
   - `WorldSnapshot` still persists `inventory` but no selected slot.
   - `SnapshotSource` still carries `inventory` but no selected slot.
   - `WorldSnapshot::capture` therefore cannot serialize the selection.
   - `validate_playable` validates inventory length/items but not a selected-slot index.
   - Backward compatibility can use `#[serde(default)]` on a `selected_hotbar_slot: usize` field; old saves then restore slot 0.
   - Validation must reject `selected_hotbar_slot >= INVENTORY_SLOT_COUNT` before accepting a snapshot as playable/restorable.

2. `src/world/save_session.rs`
   - `LastSavedState` currently tracks `inventory` but not the selected slot.
   - `capture_save_state` currently receives `Res<PlayerHotbar>` but only calls `hotbar.snapshot_items()`.
   - Add `selected_hotbar_slot: usize` to `LastSavedState`, initialize it from `hotbar.selected()`, compare it in `save_state_dirty`, and pass it into `SnapshotSource`.
   - This is required so changing only the selected slot marks the save dirty and autosave/exit-save persists it.

3. `src/screens/world_selection.rs`
   - The load path still calls `hotbar.restore_items(&snapshot.inventory)`.
   - Replace that with the already-existing atomic helper `restore_items_and_selection(&snapshot.inventory, snapshot.selected_hotbar_slot)` so invalid selection cannot partially mutate the hotbar.

## Important lifecycle invariant

Do not reintroduce the old unconditional `OnEnter(Gameplay)` selected-slot reset. New worlds are already safe because session teardown reinstalls `PlayerHotbar::default()`, while loaded worlds need their persisted selection to survive entry into Gameplay.

## Exact next implementation

Make the selected-slot change as one coherent compileable patch across the three files above:

- schema + serde default + validation + capture in `save_catalog.rs`;
- dirty-state + snapshot capture in `save_session.rs`;
- atomic restore call in `world_selection.rs`.

Then run/observe CI before expanding the refactor. Specifically verify formatting, Clippy with warnings denied, tests, and any save-fixture checks in the repository workflow. Do not claim local Windows QA unless it was actually performed.

## Connector note

Use Git blob reads for complete large-file contents. The relevant current blob SHAs at the start of this pass were:

- `src/world/save_catalog.rs`: `8e5f776c6189b2d409c27e62f5fb3a8451f060b7`
- `src/world/save_session.rs`: `4bb510edf9b2bf47fa36754ce5d107347d038bb4`
- `src/screens/world_selection.rs`: `1d091ff16e957a8c8477d6bdeee068f4621ecf92`

Before writing, re-read `develop` HEAD/blob SHAs to avoid overwriting concurrent work.
