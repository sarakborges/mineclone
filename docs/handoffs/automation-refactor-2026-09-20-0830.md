# Automation refactor handoff — 2026-09-20 08:30

## Baseline

- Branch: `develop`.
- Starting/current source checkpoint: `d2ad18f9c3f0beb46fb27960ffc1210a07606a56` (`docs: checkpoint selected hotbar slot staging work`).
- Staging branch: `automation/selected-hotbar-slot` at `f15ae78de8b84e5583b9f552e0af137b5bfdbdd1`.
- Previous checkpoint: `docs/handoffs/automation-refactor-2026-09-20-0730.md`.

## Work completed in this pass

Re-read the handoff and audited the staging branch against current `develop` rather than assuming its state. `compare_commits` reports the branches as diverged only because `develop` has the previous handoff commit: staging is exactly one source commit ahead of the merge base and its only source delta is `src/world/save_session.rs` (+7 lines), as intended. There are no competing source changes to reconcile before completing the coherent patch.

Resolved the remaining connector-read uncertainty. The staging tree reports these exact blobs:

- `src/world/save_catalog.rs`: `8e5f776c6189b2d409c27e62f5fb3a8451f060b7` (35,904 bytes)
- `src/world/save_session.rs`: `fe39dff2e1359c271cd12a1f310a122a1ed5a8d3` (15,791 bytes; contains the staged selected-slot capture work)
- `src/screens/world_selection.rs`: `1d091ff16e957a8c8477d6bdeee068f4621ecf92` (29,625 bytes)

Critically, `fetch_blob` was verified in this run to return the full 35,904-byte `save_catalog.rs` blob, not the line-truncated representation from `fetch_file`. Therefore the earlier read-truncation blocker is gone: the remaining two files can be fetched in full, transformed as complete UTF-8 text, written on staging, and then published atomically with Git tree primitives.

I also rechecked the live `save_catalog.rs` content from that full blob. The required insertion points are still unchanged: import is still `player::hotbar::INVENTORY_SLOT_COUNT`; `validate_playable` currently validates only inventory length/items; `WorldSnapshot` has `inventory` immediately followed by `fluid_updates`; `SnapshotSource` has `inventory` immediately followed by `world`; and `WorldSnapshot::capture` copies `inventory` then `fluid_updates`. No hidden schema work has landed meanwhile.

## Remaining coherent patch

Continue on `automation/selected-hotbar-slot` and finish these two files before publishing to `develop`.

### `src/world/save_catalog.rs`

1. Import `{HOTBAR_SLOT_COUNT, INVENTORY_SLOT_COUNT}`.
2. Add `#[serde(default)] pub(crate) selected_hotbar_slot: usize` immediately after `WorldSnapshot::inventory`.
3. Add `pub(crate) selected_hotbar_slot: usize` immediately after `SnapshotSource::inventory`.
4. Copy `selected_hotbar_slot: source.selected_hotbar_slot` in `WorldSnapshot::capture` immediately after `inventory`.
5. In `validate_playable`, immediately after inventory-length validation, reject `snapshot.selected_hotbar_slot >= HOTBAR_SLOT_COUNT` with invalid-data error. The hotbar bound is 9; do not use the 36-slot inventory bound. Serde default intentionally maps legacy saves to slot 0.

### `src/screens/world_selection.rs`

Replace the load-time `restore_items(...)` call with `restore_items_and_selection(&snapshot.inventory, snapshot.selected_hotbar_slot, &context.content.blocks, &context.content.tools)`, preserving the existing error propagation. The helper validates both inventory and selection before mutating live state.

## Publication strategy

Use full `fetch_blob` reads plus whole-file writes on `automation/selected-hotbar-slot`; it is safe for that branch to move through intermediate commits. Once both files are present, compare the final staging tree with `develop`. If `develop` has only documentation movement, create a tree based on current `develop` and replace the three source paths with the staging blob SHAs (or equivalently create a commit from current develop parent with those source blobs), then fast-forward `develop`. Do not fast-forward `develop` directly to staging because that would drop the handoff history on the diverged develop side.

## Verification required after coherent publication

- Confirm CI / `cargo fmt --check` and Clippy with warnings denied.
- Verify save-fixture/backward compatibility: missing `selected_hotbar_slot` deserializes as 0.
- Slot 8 accepted/restored; slot 9 rejected before live inventory mutation.
- Slot-only selection change participates in dirty detection/autosave.
- Keep the lifecycle invariant: do not restore the removed unconditional `OnEnter(Gameplay)` hotbar reset.
- Do not claim local Windows QA unless actually run.

## Important state

No source commit was added to `develop` in this pass; it remains at the prior compiling-equivalent source baseline. The staging branch still contains the intentional incomplete schema consumer in `save_session.rs`, so do not treat staging CI as meaningful until `save_catalog.rs` and `world_selection.rs` are added. The important progress of this pass is that full blob retrieval was actually exercised successfully, removing the read truncation blocker and leaving a concrete safe route to finish the two whole-file edits next.
