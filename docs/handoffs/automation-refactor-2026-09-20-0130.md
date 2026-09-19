# Automation refactor checkpoint — 2026-09-20 01:30 BRT

## Context read

Continued from `docs/handoffs/automation-refactor-2026-09-20-0100.md` on `develop`. The active persistence roadmap remains selected-hotbar-slot persistence, then player/camera look rotation, clock-only dirty-state design, then incremental chunk/region persistence.

## Implemented in this run

Observed canonical CI for the previous lifecycle commit `e810f8b8716f27c8a69e4d88de6b6b9a0229e3f8`. Rust validation run `35420283399` failed in Clippy because removing the `OnEnter(GameState::Gameplay)` reset left `app::game_state::GameState` as an unused import in `src/player/hotbar.rs`.

Commit `3f69c19a78393a4922fc7a7a7427bf45e9545970` removes that stale import. This is a compile hygiene fix only; it does not change the hotbar lifecycle behavior introduced by the prior commit.

## Verification

The failing workflow log was inspected directly and reported exactly one compiler diagnostic: unused import `app::game_state::GameState` at `src/player/hotbar.rs:6`, promoted to an error by `-D warnings`. No further compiler errors were reported before the job terminated. Immediately after publishing the fix, no GitHub Actions workflow run was yet associated with commit `3f69c19a78393a4922fc7a7a7427bf45e9545970`; a later run must confirm it green.

No `cargo test`, `cargo run`, Windows QA, PNG/GLB changes, or version bump were performed.

## Remaining selected-slot implementation

1. First observe canonical CI for `3f69c19a78393a4922fc7a7a7427bf45e9545970`; if it fails, fix the concrete diagnostic before expanding the persistence schema.
2. In `save_catalog.rs`, import `HOTBAR_SLOT_COUNT` alongside `INVENTORY_SLOT_COUNT`.
3. Add `#[serde(default)] pub(crate) selected_hotbar_slot: usize` immediately after `inventory` in `WorldSnapshot`; old saves therefore load slot 0.
4. Add `pub(crate) selected_hotbar_slot: usize` to `SnapshotSource`.
5. Reject `snapshot.selected_hotbar_slot >= HOTBAR_SLOT_COUNT` in `validate_playable()` with `invalid selected hotbar slot`.
6. Reject `source.selected_hotbar_slot >= HOTBAR_SLOT_COUNT` during `WorldSnapshot::capture()` before expensive fluid/chunk capture, then copy it into the snapshot.
7. In `save_session.rs`, add `selected_hotbar_slot: usize` to `SavedWorldState`; populate it from `captured.selected_hotbar_slot` in synchronous `persist()`, from `self.inventory.selected_slot()` in `saved_state()`, and from the captured/current hotbar in `capture_owned()`.
8. Pass `selected_hotbar_slot: self.inventory.selected_slot()` in both `SnapshotSource` construction sites.
9. In `world_selection.rs`, replace `restore_items(...)` with `restore_items_and_selection(&snapshot.inventory, snapshot.selected_hotbar_slot, ...)`.
10. Observe canonical CI for the coherent schema/wiring commit. Do not run `cargo test` without explicit authorization.
11. Once green, continue immediately with player/camera look persistence.

## Connector constraint

The GitHub connector writes complete file contents rather than applying hunks. The selected-slot schema change crosses multiple files and should be published coherently rather than leaving `develop` with only part of the field wired. This run prioritized restoring a clean CI baseline after the lifecycle change before expanding that schema.
