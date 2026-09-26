# Automation refactor checkpoint — 2026-09-20 00:00 BRT

## Context read

Continued from `docs/handoffs/automation-refactor-2026-09-19-2330.md` on `develop`. The active persistence roadmap still starts with selected-hotbar-slot persistence, followed by player/camera look rotation, clock-only dirty-state design, then incremental chunk/region persistence.

## Code completed in this run

Commit `9a0a59b9dfa423d8e0c6e95ce543949c820dc10b` (`Prepare hotbar selection restore path`) advances the selected-slot work without introducing a half-written disk schema:

- `PlayerHotbar::restore_items_and_selection(...)` now restores inventory contents and a selected slot atomically.
- The method validates `selected_slot < HOTBAR_SLOT_COUNT` before mutating the hotbar, returning `InvalidData` for corrupt values.
- Existing `restore_items(...)` delegates to the new path with slot `0`, preserving all current call sites and old behavior until the save schema/load path is wired.
- Item IDs are still fully validated before any inventory/hotbar mutation, preserving the existing all-or-nothing restore behavior.

This is intentionally a compile-compatible preparatory commit: it gives the persistence layer a validated restore target while avoiding a schema-only commit that would leave capture/load/autosave inconsistent.

## Verification

GitHub Actions run `35417348061` (`Rust validation`) was started for commit `9a0a59b9dfa423d8e0c6e95ce543949c820dc10b` and was still `in_progress` when this checkpoint was written. Do not claim it green until a later run observes `success`.

No `cargo test`, `cargo run`, Windows QA, PNG/GLB changes, or version bump were performed.

## Exact next steps

1. Re-check CI run `35417348061`; if it fails, fix that before extending the schema.
2. Add `selected_hotbar_slot` to `WorldSnapshot` with `#[serde(default)]` for backward compatibility and to `SnapshotSource`.
3. Validate `selected_hotbar_slot < HOTBAR_SLOT_COUNT` in playable snapshot validation/capture.
4. Add selected slot to `SavedWorldState`; include it in `saved_state()`, synchronous capture baseline, and owned autosave capture so a slot-only change becomes dirty.
5. Feed `PlayerHotbar::selected_slot()` through both snapshot capture paths.
6. Change `poll_world_load()` to call `restore_items_and_selection(...)` with the saved slot.
7. Remove or gate the unconditional `OnEnter(GameState::Gameplay)` selection reset. Loaded worlds must retain the restored slot; fresh worlds must still begin at slot 0 through `PlayerHotbar::default()` / explicit new-world initialization as appropriate.
8. Run/observe canonical CI: localization audit, Clippy `-D warnings`, `cargo check --locked`; do not run `cargo test` without explicit authorization.
9. Once selected-slot persistence is green, continue immediately with player/camera look persistence.

## Connector constraint

The GitHub connector replaces complete files rather than applying hunks. `save_catalog.rs` is large, so this run avoided a risky partial multi-file schema rewrite without a local checkout. The next implementation should still be done as one coherent capture/validation/load change, not as an uncompilable intermediate schema commit.