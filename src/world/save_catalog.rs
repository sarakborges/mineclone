use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::log::warn;
use serde::{Deserialize, Serialize};

use crate::{
    content::{
        block::BlockRegistry, day_night_cycle::DayNightCycleRegistry,
        dimension::DimensionRegistry, fluid::FluidRegistry, tool::ToolRegistry,
    },
    player::hotbar::INVENTORY_SLOT_COUNT,
    voxel::{chunk_disk::DiskChunk, world::VoxelWorld},
};

use super::world_names::{WORLDS_DIRECTORY, available_world_name, validate_world_name};

const SAVE_FORMAT_VERSION: u32 = 1;
const MAX_SNAPSHOT_BYTES: u64 = 512 * 1024 * 1024;
/// Count only generations that can actually be loaded into the current game.
const RETAINED_GENERATIONS: usize = 4;
// The catalog mutex only looks up an Arc. Never hold it while touching disk or
// rebuilding chunks: loading world A must not block a save of world B.
static WORLD_LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
static PRUNE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

fn world_lock(id: &str) -> io::Result<Arc<Mutex<()>>> {
    let locks = WORLD_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = locks.lock().map_err(|_| io::Error::other("world lock catalog poisoned"))?;
    Ok(Arc::clone(
        locks.entry(id.to_owned()).or_insert_with(|| Arc::new(Mutex::new(()))),
    ))
}

/// The same content definitions must validate candidates during load and
/// before old backups are removed. Invalid inventories and clocks must never
/// count toward the four recoverable backups.
#[derive(Clone, Copy)]
pub(crate) struct SaveRegistries<'a> {
    pub(crate) blocks: &'a BlockRegistry,
    pub(crate) fluids: &'a FluidRegistry,
    pub(crate) tools: &'a ToolRegistry,
    pub(crate) dimensions: &'a DimensionRegistry,
    pub(crate) cycles: &'a DayNightCycleRegistry,
}

impl SaveRegistries<'_> {
    fn validate_playable(self, snapshot: &WorldSnapshot) -> io::Result<()> {
        let duration = self
            .dimensions
            .get(&snapshot.dimension_id)
            .and_then(|dimension| self.cycles.get(&dimension.day_night_cycle))
            .map(|cycle| cycle.day_duration_ticks);
        validate_playable(snapshot, duration, |id| {
            self.blocks.get(id).is_some() || self.tools.get(id).is_some()
        })
    }

    /// Copy content lookups before starting a worker, without decoding chunks.
    pub(crate) fn owned_for_pruning(self) -> PruneRegistries {
        let valid_items = self
            .blocks
            .iter()
            .map(|block| block.id.clone())
            .chain(self.tools.iter().map(|tool| tool.id.clone()))
            .collect();
        let day_lengths = self
            .dimensions
            .iter()
            .filter_map(|dimension| {
                self.cycles
                    .get(&dimension.day_night_cycle)
                    .map(|cycle| (dimension.id.clone(), cycle.day_duration_ticks))
            })
            .collect();
        PruneRegistries {
            blocks: self.blocks.clone(),
            fluids: self.fluids.clone(),
            valid_items,
            day_lengths,
        }
    }
}

/// Owned data for detached workers; no Bevy resources are borrowed across frames.
pub(crate) struct PruneRegistries {
    blocks: BlockRegistry,
    fluids: FluidRegistry,
    valid_items: HashSet<String>,
    day_lengths: HashMap<String, u64>,
}

impl PruneRegistries {
    fn validate_playable(&self, snapshot: &WorldSnapshot) -> io::Result<()> {
        validate_playable(
            snapshot,
            self.day_lengths.get(&snapshot.dimension_id).copied(),
            |id| self.valid_items.contains(id),
        )
    }
}

fn validate_playable(
    snapshot: &WorldSnapshot,
    duration: Option<u64>,
    valid_item: impl Fn(&str) -> bool,
) -> io::Result<()> {
    if duration.is_none_or(|ticks| ticks == 0 || snapshot.tick_in_day >= ticks) {
        return Err(invalid_data("saved dimension or world clock is invalid"));
    }
    if snapshot.inventory.len() != INVENTORY_SLOT_COUNT {
        return Err(invalid_data("invalid inventory length"));
    }
    for id in snapshot.inventory.iter().flatten() {
        if !valid_item(id) {
            return Err(invalid_data(format!("unknown inventory item ID: {id}")));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct WorldManifest {
    format_version: u32,
    id: String,
    seed: u64,
    dimension_id: String,
    ticks_per_second: u32,
    last_saved_unix_ms: u64,
    #[serde(default)]
    generation: u64,
    #[serde(default)]
    snapshot_file: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct WorldSummary {
    pub(crate) id: String,
    pub(crate) last_saved_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SavedPlayer {
    pub(crate) position: [f32; 3],
    pub(crate) creative: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WorldSnapshot {
    format_version: u32,
    pub(crate) id: String,
    pub(crate) seed: u64,
    pub(crate) dimension_id: String,
    pub(crate) ticks_per_second: u32,
    pub(crate) player: Option<SavedPlayer>,
    pub(crate) day: u64,
    pub(crate) tick_in_day: u64,
    pub(crate) inventory: Vec<Option<String>>,
    chunks: Vec<DiskChunk>,
}

pub(crate) struct SnapshotSource<'a> {
    pub(crate) id: &'a str,
    pub(crate) seed: u64,
    pub(crate) dimension_id: &'a str,
    pub(crate) ticks_per_second: u32,
    pub(crate) player: Option<SavedPlayer>,
    pub(crate) day: u64,
    pub(crate) tick_in_day: u64,
    pub(crate) inventory: Vec<Option<String>>,
    pub(crate) world: &'a VoxelWorld,
    pub(crate) fluids: &'a FluidRegistry,
}

impl WorldSnapshot {
    pub(crate) fn capture(source: SnapshotSource<'_>) -> io::Result<Self> {
        validate_world_name(source.id)?;
        if source.ticks_per_second == 0 || source.dimension_id.is_empty() || source.day == 0 {
            return Err(invalid_data("incomplete world state"));
        }
        if source
            .player
            .as_ref()
            .is_some_and(|player| player.position.iter().any(|coord| !coord.is_finite()))
        {
            return Err(invalid_data("player position must be finite"));
        }
        Ok(Self {
            format_version: SAVE_FORMAT_VERSION,
            id: source.id.to_owned(),
            seed: source.seed,
            dimension_id: source.dimension_id.to_owned(),
            ticks_per_second: source.ticks_per_second,
            player: source.player,
            day: source.day,
            tick_in_day: source.tick_in_day,
            inventory: source.inventory,
            chunks: source.world.save_modified_chunks(source.fluids)?,
        })
    }
}

/// Reserve the unique directory before generating a world. Generation zero is
/// an identity marker only; Load Worlds never exposes it as a complete save.
pub(crate) fn create_new_world(
    requested_name: &str,
    seed: u64,
    dimension_id: &str,
    ticks_per_second: u32,
) -> io::Result<String> {
    if ticks_per_second == 0 || dimension_id.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "World seed metadata must include a dimension and a positive tick rate",
        ));
    }
    let root = Path::new(WORLDS_DIRECTORY);
    fs::create_dir_all(root)?;
    let mut candidate = available_world_name(requested_name)?;
    loop {
        validate_world_name(&candidate)?;
        let directory = root.join(&candidate);
        match fs::create_dir(&directory) {
            Ok(()) => {
                let manifest = WorldManifest {
                    format_version: SAVE_FORMAT_VERSION,
                    id: candidate.clone(),
                    seed,
                    dimension_id: dimension_id.to_owned(),
                    ticks_per_second,
                    last_saved_unix_ms: now_unix_ms()?,
                    generation: 0,
                    snapshot_file: None,
                };
                if let Err(error) = publish_json(&directory, &manifest_name(0), &manifest) {
                    let _ = fs::remove_file(directory.join(format!("{}.tmp", manifest_name(0))));
                    let _ = fs::remove_dir(&directory);
                    return Err(error);
                }
                return Ok(candidate);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                candidate = available_world_name(&format!("Copy of {candidate}"))?;
            }
            Err(error) => return Err(error),
        }
    }
}

/// Publish the immutable snapshot before its manifest commit marker. The
/// worker only removes backups AFTER publication, never during the write.
pub(crate) fn save_world(snapshot: &WorldSnapshot, registries: SaveRegistries<'_>) -> io::Result<u64> {
    validate_world_name(&snapshot.id)?;
    let world_lock = world_lock(&snapshot.id)?;
    let lock = world_lock.lock().map_err(|_| io::Error::other("world save lock poisoned"))?;
    if snapshot.format_version != SAVE_FORMAT_VERSION {
        return Err(invalid_data("unsupported snapshot format"));
    }
    registries.validate_playable(snapshot)?;
    let directory = Path::new(WORLDS_DIRECTORY).join(&snapshot.id);
    if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
        return Err(invalid_data("world directory cannot be a symbolic link"));
    }
    let initial: WorldManifest = read_json(&directory.join(manifest_name(0)))?;
    if initial.id != snapshot.id
        || initial.seed != snapshot.seed
        || initial.dimension_id != snapshot.dimension_id
        || initial.format_version != SAVE_FORMAT_VERSION
    {
        return Err(invalid_data("snapshot does not match reserved world identity"));
    }
    let mut next = highest_generation(&directory)?
        .checked_add(1)
        .ok_or_else(|| io::Error::other("world save generation counter exhausted"))?;
    while directory.join(snapshot_name(next)).exists()
        || directory.join(manifest_name(next)).exists()
        || directory.join(format!("{}.tmp", snapshot_name(next))).exists()
        || directory.join(format!("{}.tmp", manifest_name(next))).exists()
    {
        next = next.checked_add(1).ok_or_else(|| io::Error::other("save counter exhausted"))?;
    }
    let snapshot_file = snapshot_name(next);
    publish_json(&directory, &snapshot_file, snapshot)?;
    let manifest = WorldManifest {
        format_version: SAVE_FORMAT_VERSION,
        id: snapshot.id.clone(),
        seed: snapshot.seed,
        dimension_id: snapshot.dimension_id.clone(),
        ticks_per_second: snapshot.ticks_per_second,
        last_saved_unix_ms: now_unix_ms()?,
        generation: next,
        snapshot_file: Some(snapshot_file.clone()),
    };
    if let Err(error) = publish_json(&directory, &manifest_name(next), &manifest) {
        if let Err(cleanup_error) = fs::remove_file(directory.join(snapshot_file)) {
            warn!("Could not remove unpublished world snapshot: {cleanup_error}");
        }
        return Err(error);
    }
    let saved_at = manifest.last_saved_unix_ms;
    // Release the world's writer before scheduling cleanup. A detached worker
    // may be interrupted by process exit; this only leaves extra backups.
    drop(lock);
    if next > RETAINED_GENERATIONS as u64 {
        schedule_backup_prune(snapshot.id.clone(), registries);
    }
    Ok(saved_at)
}

fn schedule_backup_prune(id: String, registries: SaveRegistries<'_>) {
    // Bound background work: a slow cleanup must not queue an unbounded number
    // of worker threads. The next save retries a skipped cleanup.
    if PRUNE_IN_PROGRESS.swap(true, Ordering::AcqRel) {
        return;
    }
    let owned = registries.owned_for_pruning();
    let spawned = thread::Builder::new()
        .name("asteria-save-prune".to_owned())
        .spawn(move || {
            struct ResetPruneFlag;
            impl Drop for ResetPruneFlag {
                fn drop(&mut self) {
                    PRUNE_IN_PROGRESS.store(false, Ordering::Release);
                }
            }
            let _reset = ResetPruneFlag;
            let directory = Path::new(WORLDS_DIRECTORY).join(&id);
            if let Err(error) = prune_old_generations(&directory, &id, &owned) {
                warn!("World {id} was saved, but background backup cleanup failed: {error}");
            }
        });
    if let Err(error) = spawned {
        PRUNE_IN_PROGRESS.store(false, Ordering::Release);
        warn!("World was saved, but backup cleanup thread could not start: {error}");
    }
}

/// Enumerate only fully published generations. Missing or temporary snapshots
/// never become selectable worlds. Metadata-only; never assert restorable here.
pub(crate) fn list_worlds() -> io::Result<Vec<WorldSummary>> {
    let entries = match fs::read_dir(WORLDS_DIRECTORY) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut worlds = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let Some(id) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if validate_world_name(&id).is_err() {
            continue;
        }
        let world_lock = world_lock(&id)?;
        let _lock = world_lock.lock().map_err(|_| io::Error::other("world save lock poisoned"))?;
        if let Ok(manifest) = latest_complete_manifest(&entry.path(), &id) {
            worlds.push(WorldSummary { id, last_saved_unix_ms: manifest.last_saved_unix_ms });
        }
    }
    worlds.sort_unstable_by(|a, b| {
        b.last_saved_unix_ms.cmp(&a.last_saved_unix_ms).then_with(|| a.id.cmp(&b.id))
    });
    Ok(worlds)
}

/// Expensive: only call from the world-selection worker, never from a Bevy
/// system. Reuses the exact snapshot and gameplay validation used by pruning.
pub(crate) fn list_verified_worlds(registries: &PruneRegistries) -> io::Result<Vec<WorldSummary>> {
    let candidates = list_worlds()?;
    let mut verified = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        match newest_restorable_timestamp(&candidate.id, registries) {
            Ok(timestamp) => verified.push(WorldSummary {
                id: candidate.id,
                last_saved_unix_ms: timestamp,
            }),
            Err(error) => warn!("World {} has no verified snapshot: {error}", candidate.id),
        }
    }
    verified.sort_unstable_by(|a, b| {
        b.last_saved_unix_ms.cmp(&a.last_saved_unix_ms).then_with(|| a.id.cmp(&b.id))
    });
    Ok(verified)
}

fn newest_restorable_timestamp(id: &str, registries: &PruneRegistries) -> io::Result<u64> {
    validate_world_name(id)?;
    // Hold only this world's lock across enumeration and reconstruction.
    // Backup pruning cannot unlink its fallback until validation finishes.
    let world_lock = world_lock(id)?;
    let _lock = world_lock.lock().map_err(|_| io::Error::other("world save lock poisoned"))?;
    let directory = Path::new(WORLDS_DIRECTORY).join(id);
    if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
        return Err(invalid_data("world directory cannot be a symbolic link"));
    }
    let mut candidates = manifest_paths(&directory)?;
    candidates.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
    for (generation, path) in candidates {
        let Ok(manifest) = read_json::<WorldManifest>(&path) else {
            continue;
        };
        if !valid_manifest(&manifest, id, generation) {
            continue;
        }
        match load_snapshot(
            &directory,
            id,
            &manifest,
            &registries.blocks,
            &registries.fluids,
            |snapshot| registries.validate_playable(snapshot),
        ) {
            Ok(_) => return Ok(manifest.last_saved_unix_ms),
            Err(error) => warn!("Skipping damaged save for world {id}, generation {generation}: {error}"),
        }
    }
    Err(invalid_data(format!("world {id} has no restorable save")))
}

/// Try each published generation newest first. All gameplay-visible state must
/// validate before accepting a candidate so the next older snapshot is tried
/// for an invalid inventory or clock as well as corrupt chunk data.
pub(crate) fn load_world(
    id: &str,
    registries: SaveRegistries<'_>,
) -> io::Result<(WorldSnapshot, VoxelWorld)> {
    validate_world_name(id)?;
    // Lock this world's enumeration and reconstruction only. The prune worker
    // also takes this lock before deleting its old manifests and snapshots.
    let world_lock = world_lock(id)?;
    let _lock = world_lock.lock().map_err(|_| io::Error::other("world save lock poisoned"))?;
    let directory = Path::new(WORLDS_DIRECTORY).join(id);
    if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
        return Err(invalid_data("world directory cannot be a symbolic link"));
    }
    let mut candidates = manifest_paths(&directory)?;
    candidates.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
    let mut last_error = None;
    for (generation, path) in candidates {
        let Ok(manifest) = read_json::<WorldManifest>(&path) else {
            continue;
        };
        if !valid_manifest(&manifest, id, generation) {
            continue;
        }
        match load_snapshot(
            &directory,
            id,
            &manifest,
            registries.blocks,
            registries.fluids,
            |snapshot| registries.validate_playable(snapshot),
        ) {
            Ok(loaded) => return Ok(loaded),
            Err(error) => {
                warn!("Skipping damaged save for world {id}, generation {generation}: {error}");
                last_error = Some(error);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| invalid_data(format!("world {id} has no restorable save"))))
}

fn load_snapshot(
    directory: &Path,
    id: &str,
    manifest: &WorldManifest,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    validate: impl FnOnce(&WorldSnapshot) -> io::Result<()>,
) -> io::Result<(WorldSnapshot, VoxelWorld)> {
    let filename = manifest.snapshot_file.as_ref().ok_or_else(|| invalid_data("no complete snapshot"))?;
    let expected = snapshot_name(manifest.generation);
    if filename != &expected {
        return Err(invalid_data("manifest references a noncanonical snapshot path"));
    }
    let path = directory.join(filename);
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_SNAPSHOT_BYTES {
        return Err(invalid_data("snapshot is not a regular file or exceeds supported size"));
    }
    let mut snapshot: WorldSnapshot = read_json(&path)?;
    if snapshot.format_version != SAVE_FORMAT_VERSION
        || snapshot.id != id
        || snapshot.seed != manifest.seed
        || snapshot.dimension_id != manifest.dimension_id
        || snapshot.ticks_per_second != manifest.ticks_per_second
        || snapshot.ticks_per_second == 0
        || snapshot.day == 0
        || snapshot
            .player
            .as_ref()
            .is_some_and(|player| player.position.iter().any(|coord| !coord.is_finite()))
    {
        return Err(invalid_data("snapshot metadata or player state is invalid"));
    }
    validate(&snapshot)?;
    let world = VoxelWorld::from_saved_chunks(std::mem::take(&mut snapshot.chunks), blocks, fluids)?;
    Ok((snapshot, world))
}

fn valid_manifest(manifest: &WorldManifest, id: &str, generation: u64) -> bool {
    manifest.format_version == SAVE_FORMAT_VERSION
        && manifest.id == id
        && manifest.generation == generation
        && manifest.ticks_per_second > 0
        && generation > 0
        && manifest.snapshot_file.as_deref() == Some(snapshot_name(generation).as_str())
}

fn latest_complete_manifest(directory: &Path, id: &str) -> io::Result<WorldManifest> {
    let mut candidates = manifest_paths(directory)?;
    candidates.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
    for (generation, path) in candidates {
        let Ok(manifest) = read_json::<WorldManifest>(&path) else {
            continue;
        };
        if valid_manifest(&manifest, id, generation)
            && directory.join(snapshot_name(generation)).is_file()
        {
            return Ok(manifest);
        }
    }
    Err(invalid_data(format!("world {id} has no published complete save")))
}

/// Validate backups off-thread and prune only when four generations decode.
/// The writer only appends immutable generations. The deletion phase locks
/// this world only, so other worlds remain writable during a slow load.
fn prune_old_generations(directory: &Path, id: &str, registries: &PruneRegistries) -> io::Result<()> {
    if !fs::symlink_metadata(directory)?.file_type().is_dir() {
        return Err(invalid_data("world directory cannot be a symbolic link"));
    }
    let mut candidates = manifest_paths(directory)?;
    if candidates.len() <= RETAINED_GENERATIONS + 1 {
        return Ok(());
    }
    candidates.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
    let mut restorable = 0;
    let mut cutoff = None;
    for (generation, path) in &candidates {
        let Ok(manifest) = read_json::<WorldManifest>(path) else {
            continue;
        };
        if !valid_manifest(&manifest, id, *generation) {
            continue;
        }
        match load_snapshot(
            directory,
            id,
            &manifest,
            &registries.blocks,
            &registries.fluids,
            |snapshot| registries.validate_playable(snapshot),
        ) {
            Ok(_) => {
                restorable += 1;
                if restorable == RETAINED_GENERATIONS {
                    cutoff = Some(*generation);
                    break;
                }
            }
            Err(error) => {
                warn!("Preserving older saves because world {id} generation {generation} is not restorable: {error}");
            }
        }
    }
    let Some(cutoff) = cutoff else {
        return Ok(());
    };
    // Expensive validation is complete: lock only this world's deletion phase.
    // Saves append immutable generations and cannot invalidate the cutoff.
    let world_lock = world_lock(id)?;
    let _lock = world_lock.lock().map_err(|_| io::Error::other("world save lock poisoned"))?;
    if !fs::symlink_metadata(directory)?.file_type().is_dir() {
        return Err(invalid_data("world directory cannot be a symbolic link"));
    }
    // Remove each manifest before its snapshot: process termination during
    // cleanup cannot leave a manifest pointing at a deleted snapshot.
    for (generation, path) in candidates {
        if generation > 0 && generation < cutoff {
            fs::remove_file(path)?;
        }
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(generation) = parse_generation(&name, "snapshot-") else {
            continue;
        };
        if generation > 0 && generation < cutoff {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn highest_generation(directory: &Path) -> io::Result<u64> {
    Ok(manifest_paths(directory)?
        .into_iter()
        .map(|(generation, _)| generation)
        .max()
        .unwrap_or(0))
}

fn manifest_paths(directory: &Path) -> io::Result<Vec<(u64, PathBuf)>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if let Some(generation) = parse_generation(&name, "manifest-") {
            result.push((generation, entry.path()));
        }
    }
    Ok(result)
}

fn parse_generation(name: &str, prefix: &str) -> Option<u64> {
    let digits = name.strip_prefix(prefix)?.strip_suffix(".json")?;
    (digits.len() == 20 && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

fn manifest_name(generation: u64) -> String {
    format!("manifest-{generation:020}.json")
}

fn snapshot_name(generation: u64) -> String {
    format!("snapshot-{generation:020}.json")
}

/// Cap serialized JSON while writing, rather than creating a potentially
/// 512-MiB Vec just to reject it after serialization. Include the final newline.
struct SnapshotSizeLimit<W> {
    writer: W,
    remaining: u64,
}

impl<W: Write> Write for SnapshotSizeLimit<W> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.len() as u64 > self.remaining {
            return Err(invalid_data("snapshot exceeds the maximum supported size"));
        }
        let written = self.writer.write(data)?;
        self.remaining -= written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

fn publish_json<T: Serialize>(directory: &Path, filename: &str, value: &T) -> io::Result<()> {
    let temporary = directory.join(format!("{filename}.tmp"));
    let final_path = directory.join(filename);
    let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
    let result = (|| {
        {
            let mut buffered = io::BufWriter::new(&mut file);
            if filename.starts_with("snapshot-") {
                let mut bounded = SnapshotSizeLimit {
                    writer: &mut buffered,
                    remaining: MAX_SNAPSHOT_BYTES,
                };
                serde_json::to_writer(&mut bounded, value).map_err(io::Error::other)?;
                bounded.write_all(b"\n")?;
            } else {
                serde_json::to_writer(&mut buffered, value).map_err(io::Error::other)?;
                buffered.write_all(b"\n")?;
            }
            buffered.flush()?;
        }
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &final_path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> io::Result<T> {
    // Avoid a second allocation as large as the on-disk JSON. The deserialized
    // snapshot still owns its chunks, and callers continue validating them.
    serde_json::from_reader(io::BufReader::new(fs::File::open(path)?)).map_err(io::Error::other)
}

fn now_unix_ms() -> io::Result<u64> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).map_err(io::Error::other)?;
    u64::try_from(elapsed.as_millis()).map_err(io::Error::other)
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
