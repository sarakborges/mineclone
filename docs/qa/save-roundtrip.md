# Save and restore: Windows QA protocol

**Status: not executed.** This is a manual verification protocol, not evidence that any scenario passed. Use an isolated, disposable `worlds/` tree; do not corrupt your only copy of a real world. `worlds/` is ignored by Git. Record the commit SHA, `VERSION`, Windows build, filesystem, game log and observed versus expected results for every run. Never interpret green Linux CI as Windows/runtime validation.

## Preparation and measurements

1. Make a backup of existing `worlds/` **with the game closed**. Use a disposable working directory or temporarily rename `worlds` to `worlds-backup-YYYYMMDD` before starting; do not overwrite an existing backup. Restore it only after quitting the game.
2. Launch the checked-out `develop` build on Windows. Capture the complete console/game log at info level. Record elapsed wall-clock time from user action to visible result and use Task Manager for peak process RAM, CPU and disk pressure. Mark unmeasured fields as *not measured*, never as zero.
3. For **saves**, copy the exact `World <id> saved: capture=..., publication=...` log. `capture` includes capturing modified chunks; `publication` includes JSON write, fsync, manifest and cleanup dispatch. They are both synchronous and neither is a whole-frame profiler.
4. For **world selection**, copy `Saved-world catalog definitions copied on main thread: ...` and `Saved-world catalog verification: duration=..., successful=..., worlds=...`. For **loading**, copy `World <id> load definitions copied on main thread: ...` and `World <id> load worker: duration=..., successful=...`. These logs are emitted by code from `c05e5a7` onward. A worker duration does **not** include main-thread definition copying, menu transition or full user-perceived wait. Match log lines to the same world/run; record apparent frame stalls separately.
5. Use three separate worlds: a small new world, one with edits spread across many chunks (including chunks moved out of render distance), and one with multiple generations/large saved data. Record manifest/snapshot file names and sizes **only while the game is closed**, unless verifying concurrency explicitly.
6. Confirm `cargo check --locked` and Clippy `--all-targets --all-features -- -D warnings` are green for the exact commit. Do not add or run `cargo test` without authorization.

## Core persistence and restart

| Case | Actions | Expected result |
| --- | --- | --- |
| Identity | Create two worlds with the same requested name, using different seeds; open each again. Also try `COM¹`, `LPT²`, `COM³.txt`, `CONIN$`, `CONOUT$` and valid Unicode names at the 200 UTF-16-unit limit; duplicate a maximum-length world name twice. | Second world has a unique name; neither world overwrites the other's directory or data. Windows device names are rejected before attempting to create a directory. Numeric copy names fit within the limit; simultaneous `create_dir AlreadyExists` remains a separate race to test. |
| Blocks, fluids and paint | Modify and paint identifiable blocks, place/remove fluid, move far enough to archive the chunks, save via Leave World and quit the process; restart and return. | Edited blocks, fluid state and brush color/properties survive archiving and a complete process restart in the correct positions. |
| Player state | Set distinctive inventory slots, creative/survival mode, position and tick rate; save, quit and restart. | Inventory, mode, coordinates and tick rate match the saved values; no cross-world leakage. |
| Clock | Record day/time immediately before saving, quit/restart, and reopen. | Restored saved day/time is valid for the dimension's cycle, then advances normally. Pause must not freeze world simulation. |
| Autosave | Save baseline, change exactly one persisted property, keep playing over 60 s, close and reopen. Repeat for block/fluid changes and inventory/position independently. | A new committed generation appears for changes; no extra generation arises solely from idle clock or worldgen streaming. |
| Leave/Exit failure | In a disposable world, induce a write error (e.g. read-only directory) before Leave/Exit. | An error is surfaced, the game/world remains active, and last committed generation is loadable after restoring permissions. Never edit permissions on the only real save. |
| Frame cost | Compare small and large worlds, including archived edits; collect actual `capture`/`publication` logs during autosave, Leave and Exit, and compare main-thread definition-copy time against scan/load worker times separately. | Determine measured bottleneck; record visible stalls, CPU, RAM and disk use, not an assumed performance improvement. |

## Corruption, fallback and interrupted publication

All changes in this section must be made to **copies** of disposable worlds while the game is closed. Keep the original snapshot and manifest files for restoration.

### Safe fixture generator

`tools/make_save_fixture.py` constructs a separate output tree for small, disposable worlds with at least two committed generations. It refuses an existing output directory, symlink-containing source worlds and source snapshots larger than 64 MiB (a QA-tool limit, **not** the game's 512 MiB save limit). Example from the repository root, with the game closed:

```powershell
py -3 tools/make_save_fixture.py "worlds\Demo" "qa-fixture-inventory" --damage inventory-id
```

The script produces `qa-fixture-inventory/worlds/Demo` and never modifies `worlds/Demo`. Back up and close the game, then use the generated folder **only in a separate disposable game working tree** (or temporarily substitute it for an already backed-up disposable world). Do not copy it over a real world's only copy; close the game before swapping again. Use a *new* output directory for each mode. Available modes are `snapshot-json`, `manifest-json`, `inventory-id`, `clock`, `duplicate-chunk` (requires an edited/saved chunk), and `player-null`. The first five should exercise documented corruption/fallback behavior; **`player-null` is exploratory**, not a passing criterion until legacy save compatibility and player-state requirements are resolved. Record whether a null player is accepted or skipped, and whether position/mode survive; do not assume recovery. The generator has been checked with synthetic copies only, not with actual Windows game saves.

1. Create at least five complete save generations and note their timestamps. Damage only the newest *snapshot JSON* (for example, replace it with malformed JSON) and restart. Load Worlds must show the timestamp of the newest **recoverable** older generation, and Load must restore that generation. Record the warning.
2. Restore the fixture; corrupt the newest snapshot's inventory item ID, inventory slot count, `tick_in_day`, dimension ID, player position, and chunk/block/fluid data in separate experiments. Include `duplicate-chunk` from the generator, check that fallback happens without applying any chunk of the failed generation, and retain older valid snapshots. Each established invalid case must fall back to a complete older generation rather than applying partial state. A fully unrestoreable world must not appear as playable in the selection list.
3. Simulate interruption between snapshot and manifest publication by placing an uncommitted `snapshot-<generation>.json` without its matching manifest, using an isolated fixture. It must not supersede a committed generation. Repeat with a malformed manifest and with a temporary `*.tmp` file.
4. Simulate interruption during pruning by removing only an old manifest while retaining its snapshot in a copied fixture. The older orphan must not be displayed as a complete save, and the newer valid generations must remain playable. Check that retention does not delete the last four actually recoverable generations.
5. Create an oversized snapshot fixture (larger than the supported 512 MiB limit) only if disk space is adequate. A load must reject it and fall back; a new save that exceeds the limit must not publish its manifest. Avoid filling the system disk; the fixture generator intentionally refuses this size, so use another disposable procedure.

## Worker, lock and navigation races

1. In a large disposable world, click Load then Back immediately. Repeat while a completed load is being processed, and after reopening the selection screen. Back must win, and an abandoned worker must never start Gameplay or overwrite active world resources. A new load may be blocked until the abandoned worker finishes, but must eventually work. Check that RAM returns after an abandoned result is discarded even if the menu is never reopened.
2. While the selection scan is verifying a large world, leave and reenter the screen. Check for one reusable scan (not an unbounded family of workers), correct buttons, no duplicated entries and an accurate timestamp. Load cannot proceed until verification has completed.
3. While a reader is reconstructing a world, trigger another save and pruning of **that same disposable world** if the game permits it. The writer must not wait for JSON/chunk decoding solely due to the reader lease; the pruning worker may wait outside the write lock and must automatically resume after the final reader releases its lease. No fallback paths may disappear while read. Record actual behavior and timings; do not assert that this race was covered by CI.
4. Run saves on two different disposable worlds as independently as the UI permits. A load/scan of world A must not directly hold world B's gate; observe any CPU/disk contention separately from mutex contention.
5. Produce many generations, including invalid ones, to verify the reader opens no more than one snapshot at a time. Check handle count, memory consumption and whether pruning retains four recoverable generations plus any temporarily needed fallbacks. Save again while cleanup is already running and inspect whether the newest generation is eventually considered; a skipped cleanup request may leave extra backups until a later save. Close/reopen the program and confirm no stranded worker or undeletable files remain.
6. If a loader never completes, record the situation instead of waiting indefinitely: a cleanup worker currently has no reader timeout/cancellation, so an unfinished reader can delay that world's pruning. Preserve all saved files for investigation.

## Evidence and exit criteria

For each case: record `PASS`, `FAIL`, or `NOT RUN`; exact steps, commit, OS/filesystem, fixture generation, logs, timings, and screenshots only where helpful. A failure requires the smallest reproducible fixture and the relevant error; do not delete the fixture until the issue is understood. **Do not change root `VERSION` or mark saves production-ready solely on Clippy, `cargo check`, or this checklist.** Finish and review genuine Windows game/restart QA first.
