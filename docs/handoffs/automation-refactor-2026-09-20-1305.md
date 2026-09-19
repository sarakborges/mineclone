# Automation refactor checkpoint — clock persistence

## Published on `develop`

- `0375f94bfe04b51ab952985ee2c0a1b650c0bc25` is the rebase-merge result of PR #15 (`automation/clock-persistence`).
- World clock progress now has lightweight immutable `clock-*.json` checkpoints every 60 seconds instead of making clock-only passage dirty the full global snapshot.
- Restore first uses the validated immutable snapshot clock, then applies a sidecar checkpoint only when it is later and valid for the active day/night cycle. Stale/corrupt/invalid checkpoints cannot roll the snapshot clock backwards.
- Clock checkpoint publication is create-new + fsync + rename; older checkpoints are pruned only after the new file is published.
- Also removed obsolete `ChunkRenderPool::active_coords`, which became dead after the render/fog changes already present on `develop`.

## Validation

PR #15 Rust validation run `35444646854`: Clippy succeeded and `cargo check` succeeded before merge. The earlier run `35444571395` exposed the unrelated obsolete `active_coords`; that was fixed in staging and revalidated.

## Continuation

Created staging `automation/worldgen-version-persistence` from current `develop` for the next persistence item.

Current persistence behavior in `src/voxel/world/persistence.rs` intentionally stores only chunks with persistent mutations; untouched deterministic terrain is regenerated from seed after load. This means a world-generation algorithm/content change can change untouched areas of an existing world. Next step: trace generation identity/version ownership and implement a persisted worldgen compatibility identity without duplicating authoritative world state. Do not solve this by snapshotting every generated chunk; that would regress the global snapshot scalability goal. After that, continue toward incremental chunk/region persistence rather than expanding monolithic snapshots.
