# Automation refactor handoff — 2026-09-19 18:20 BRT

## Canonical baseline

- Branch: `develop`.
- HEAD at checkpoint start: `b87dff86a29d055a38edb6c37f0cd87d63a88381` (`Bump version for worldgen compatibility persistence`).
- `VERSION`: `0.34.7`.
- Rust validation run `35469535814` for this HEAD completed successfully.
- `ARCHITECTURE.md` remains authoritative.

## Reconciliation performed

The old top section of `HANDOFF.md` is stale relative to current `develop`. Before selecting new work, the automation inspected the outstanding `automation/*` branches and compared them against current `develop`.

- `automation/player-look-persistence`: fully behind current `develop`; its feature is already integrated.
- `automation/selected-hotbar-slot`: still has three commits ahead of its old merge base, but current `develop` already contains the intended behavior: `WorldSnapshot.selected_hotbar_slot` with serde fallback, HOTBAR-bound validation, capture through save state, dirty detection, and load-time restoration. The branch is therefore historical/stale, not pending implementation.
- `automation/clock-persistence`: diverged from current `develop`, but current `develop` already contains `src/world/clock_persistence.rs` and its `WorldPlugin` integration. The branch is likewise historical/stale.
- `automation/worldgen-version-persistence`: merged through PR #16 / merge commit `01a2468142a6f521df76bbd341297545bcd578bd`; version bump is `b87dff86...`.

A temporary draft PR #17 was opened only to test whether the old selected-slot branch still represented unapplied work. GitHub reported it non-mergeable; after source inspection proved the feature already exists in `develop`, PR #17 was closed without merge. Do not revive or merge that stale branch.

## Current save-state status

The previously listed small persistence gaps are now implemented in `develop`:

- selected hotbar slot persists/restores and participates in dirty-state detection;
- player yaw/pitch persist/restore with validation;
- lightweight clock-only checkpoints exist outside the full world snapshot;
- worldgen compatibility identity is persisted in generation-zero manifest and snapshots, with legacy fallback and manifest/snapshot matching.

Worldgen identity deliberately protects regeneration compatibility, but it does not solve the structural storage limitation below.

## Next executable roadmap item

The remaining architectural save refactor is the large one already identified in `HANDOFF.md`: migrate generated/explored chunk persistence away from the global JSON snapshot into incremental chunk/region storage.

Required invariants from the existing architecture/handoff:

1. A generated chunk becomes authoritative after it exists; unloading must not require rerunning worldgen for explored terrain.
2. Do not retain all explored chunks in RAM.
3. Metadata/player/entity/scheduled-work commits must remain recoverable and atomic.
4. Chunk storage must support partial/incremental updates instead of rewriting every persistent chunk on each autosave.
5. Existing generation fallback/recovery semantics must not silently accept incompatible or corrupt chunk state.
6. Preserve the current cross-process world-directory lock and immutable-generation recovery guarantees while introducing segmented chunk storage.
7. Do not add or run `cargo test` without explicit authorization. CI validation remains localization audit + Clippy `-D warnings` + `cargo check --locked`.

### Recommended first implementation slice

Do not attempt the whole migration in one commit. Start by separating the serialization boundary without changing runtime semantics:

- extract chunk persistence from `WorldSnapshot` behind a storage/catalog abstraction owned by the world save layer;
- define a stable per-chunk disk identity and validation path using existing `DiskChunk` encoding;
- keep the current snapshot path as the compatibility reader while new writes can be staged incrementally;
- only switch unload/load ownership after the new storage path has CI-clean validation and recovery behavior.

This slice should stay on a fresh `automation/*` staging branch until coherent and validated; do not overwrite `develop` with a partially migrated save format.

## Validation facts

- Current `develop` CI: green (`35469535814`).
- No `cargo test`, `cargo run`, or Windows QA was executed in this checkpoint.
