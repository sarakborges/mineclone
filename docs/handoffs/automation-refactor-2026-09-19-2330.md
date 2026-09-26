# Automation refactor checkpoint — 2026-09-19 23:30 BRT

## Context read

Active source is `sarakborges/mineclone`, branch `develop`. The current `HANDOFF.md` resumes the save-game persistence audit after checkpoint 118. The first pending functional item is persistence of the selected hotbar slot, followed by player/camera look rotation, clock-only dirty-state design, and then incremental chunk/region persistence.

## Audit performed in this run

Selected-hotbar persistence was traced end-to-end before changing the save schema:

- `src/player/hotbar.rs`: `PlayerHotbar` owns `selected_slot`; `saved_items()` serializes only item contents; `restore_items()` restores the 36 inventory entries and unconditionally sets `selected_slot = 0`.
- `src/player/hotbar.rs`: `PlayerHotbarPlugin` also runs `reset_hotbar_selection` on every `OnEnter(GameState::Gameplay)`, so restoring a slot earlier in world selection would be overwritten unless this lifecycle reset is changed.
- `src/world/save_catalog.rs`: `WorldSnapshot` currently stores `inventory` but has no selected-slot field. Backward compatibility should use a serde default of zero and validation must reject values outside `0..HOTBAR_SLOT_COUNT`.
- `src/world/save_session.rs`: `SavedWorldState` currently tracks inventory contents but not selection, so merely adding the field to the disk snapshot would still fail to make a slot-only change dirty for autosave. Both synchronous and owned captures must include the selected slot in snapshot and baseline state.
- `src/screens/world_selection.rs`: load currently calls `restore_items()` before transitioning through Loading to Gameplay. The selected slot should be restored through the same validated hotbar restore path, but the Gameplay `OnEnter` reset must not overwrite loaded state.

## Repository write in this run

Commit `13c79dc3c0cd80863ab1ccbde9679de5af5288a3` was created while verifying connector write semantics for `src/player/hotbar.rs`; its blob SHA remained `64ef59e5394e53f1d87b28fc63447b268fc02770`, so it is intentionally a no-content-change checkpoint and introduces no runtime behavior change.

No functional save-schema change was committed in this run because the GitHub connector's file update operation requires complete-file replacement and the selected-slot change spans several large files. A partial schema change would leave `develop` uncompilable or silently incomplete, so it was not attempted.

## Exact implementation plan for next run

1. Add a backward-compatible selected-slot field to `WorldSnapshot` (serde default zero) and to `SnapshotSource`.
2. Validate selected slot `< HOTBAR_SLOT_COUNT` during playable snapshot validation/capture.
3. Add selected slot to `SavedWorldState` so changing only the active slot marks autosave dirty.
4. Feed `PlayerHotbar::selected_slot()` through synchronous and owned save captures.
5. Extend the hotbar restore API to restore inventory plus validated selected slot atomically.
6. Remove/replace the unconditional Gameplay-entry reset so new worlds still start at slot 0 while loaded worlds retain the restored slot.
7. Update `poll_world_load()` to restore the snapshot's selected slot.
8. Run the canonical CI path only: localization audit, Clippy `-D warnings`, and `cargo check --locked`; do not run `cargo test` without explicit authorization.
9. If green, update `HANDOFF.md` with commit SHA/CI and continue immediately to player/camera look persistence.

## Constraints preserved

No `cargo test`, `cargo run`, Windows QA, PNG/GLB changes, version bump, or claims of runtime roundtrip were made in this run.
