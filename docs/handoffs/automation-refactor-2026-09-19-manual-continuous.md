# Automation refactor handoff — 2026-09-19 manual continuous run

## Baseline

- Branch: `develop`.
- Selected-hotbar persistence source commit: `b627751e0b66310c289c9d4468498feb2031aa5f`.
- CI exposed one follow-up dead-code failure; fixed by `874c215eef38e82915be746fb62827e8542e7b6b` (`refactor: remove obsolete inventory restore wrapper`).
- GitHub Actions run `35442110857` for `874c215e...` was completed / success when this handoff was written.

## Work completed

The selected-hotbar roadmap item is now implemented coherently on `develop`:
- snapshot schema persists `selected_hotbar_slot` with `#[serde(default)]`, preserving legacy saves as slot 0;
- validation uses `HOTBAR_SLOT_COUNT` (slot 8 valid, slot 9 invalid);
- save/autosave dirty state captures slot-only changes;
- world load uses `restore_items_and_selection`, validating inventory + selection before mutation;
- obsolete `restore_items` wrapper was removed after Clippy correctly reported it as dead code.

The implementation preserves the architecture rule against wrappers that merely call another helper without domain semantics.

## Architecture/roadmap continuation

Read `ARCHITECTURE.md` and `HANDOFF.md`. The next explicit save-game roadmap item is player/camera rotation/look persistence.

Current ownership was mapped:
- `src/player/camera.rs`: `GameplayCamera { yaw, pitch }` is the authoritative look state.
- `src/player/camera/look.rs`: mouse look mutates `GameplayCamera.yaw/pitch` and derives `Transform.rotation` from them.
- `src/world/save_session.rs`: the save query already selects the same player entity `With<GameplayCamera>`, but currently queries only `PlayerId, Transform, GameMode, EntityHealth`.
- `src/world/save_catalog.rs`: `SavedPlayer` currently stores position, creative and optional health.
- `src/player/save.rs`: `PlayerSaveData` currently stores position/game mode/health; inspect its spawn/load consumers before deciding whether restored yaw/pitch should flow through this type or another existing lifecycle path.

Architectural constraint: do not create a second persistent look-state resource. Persist the authoritative `GameplayCamera` yaw/pitch and restore it so `Transform.rotation` is derived consistently from the same values.

## Next executable steps

1. Wait/check CI run `35442110857`; if it fails, fix that first.
2. Trace `PlayerSaveData` consumers and player/camera spawn path.
3. Implement backward-compatible yaw/pitch fields in `SavedPlayer` (serde defaults must preserve old saves).
4. Validate finite yaw/pitch and clamp/validate pitch consistently with camera look invariants; avoid duplicating magic bounds if a shared domain primitive is appropriate.
5. Include yaw/pitch in `SavedWorldState` so look-only changes participate in autosave dirty detection.
6. Capture yaw/pitch from authoritative `GameplayCamera`, not by reverse-decomposing the quaternion.
7. Restore `GameplayCamera` + derived `Transform.rotation` atomically during player activation.
8. Run CI and continue immediately to the next roadmap item (clock-only dirty-state / incremental chunk persistence) if green.

Do not re-audit selected-hotbar persistence unless relevant code changes. Continue implementation directly from this handoff.
