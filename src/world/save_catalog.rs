use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::log::warn;
use serde::{Deserialize, Serialize};

use crate::{
    content::{block::BlockRegistry, fluid::FluidRegistry},
    voxel::{chunk_disk::DiskChunk, world::VoxelWorld},
};

use super::world_names::{WORLDS_DIRECTORY, available_world_name, validate_world_name};

const SAVE_FORMAT_VERSION: u32 = 1;
const MAX_SNAPSHOT_BYTES: u64 = 512 * 1024 * 1024;
/// Retain the newest save and three predecessors so a damaged recent snapshot
/// does not immediately destroy the last known good generation.
const RETAINED_GENERATIONS: usize = 4;
static SAVE_LOCK: Mutex<()> = Mutex::new(());

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
        if source.player.as_ref().is_some_and(|player| {
            player.position.iter().any(|coord| !coord.is_finite())
        }) {
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

/// Create and reserve the ID *before* world generation. Initial metadata is
/// intentionally not a complete save; only published snapshots appear in Load Worlds.
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

/// Publish a complete immutable snapshot first, followed by its manifest as
/// the commit marker. A crash before manifest publication leaves the previous
/// complete generation usable; the next writer never overwrites an old file.
pub(crate) fn save_world(snapshot: &WorldSnapshot) -> io::Result<u64> {
    let _lock = SAVE_LOCK.lock().map_err(|_| io::Error::other("world save lock poisoned"))?;
    validate_world_name(&snapshot.id)?;
    if snapshot.format_version != SAVE_FORMAT_VERSION {
        return Err(invalid_data("unsupported snapshot format"));
    }
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
        // No commit marker was published: the new file is an orphan, not a save.
        if let Err(cleanup_error) = fs::remove_file(directory.join(snapshot_file)) {
            warn!("Could not remove unpublished world snapshot: {cleanup_error}");
        }
        return Err(error);
    }
    // Publication succeeded. Pruning is best effort: a cleanup error must not
    // turn an already committed save into an apparent save failure.
    if let Err(error) = prune_old_generations(&directory, &snapshot.id) {
        warn!("World save committed, but old snapshot cleanup failed: {error}");
    }
    Ok(manifest.last_saved_unix_ms)
}

/// Return only valid, completed manifests. Incomplete world creation and
/// temporary files cannot masquerade as a world available for loading.
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
        if let Ok(manifest) = latest_complete_manifest(&entry.path(), &id) {
            worlds.push(WorldSummary { id, last_saved_unix_ms: manifest.last_saved_unix_ms });
        }
    }
    worlds.sort_unstable_by(|a, b| {
        b.last_saved_unix_ms.cmp(&a.last_saved_unix_ms).then_with(|| a.id.cmp(&b.id))
    });
    Ok(worlds)
}

/// Try generations newest first, validating the entire snapshot and its block
/// and fluid registries before returning a world. A damaged newest generation
/// must not hide an older, restorable save.
pub(crate) fn load_world(
    id: &str,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> io::Result<(WorldSnapshot, VoxelWorld)> {
    validate_world_name(id)?;
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
        match load_snapshot(&directory, id, &manifest, blocks, fluids) {
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
        || snapshot.player.as_ref().is_some_and(|player| {
            player.position.iter().any(|coord| !coord.is_finite())
        })
    {
        return Err(invalid_data("snapshot metadata or player state is invalid"));
    }
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

/// Prune only after a newer manifest has been committed, and only when four
/// published generations exist. Preserve generation zero (the reserved ID).
/// Manifest removal precedes snapshot removal so interrupted pruning cannot
/// expose a dangling entry in the world catalogue.
fn prune_old_generations(directory: &Path, id: &str) -> io::Result<()> {
    let mut candidates = manifest_paths(directory)?;
    candidates.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
    let mut published = 0;
    let mut cutoff = None;
    for (generation, path) in &candidates {
        let Ok(manifest) = read_json::<WorldManifest>(path) else {
            continue;
        };
        if valid_manifest(&manifest, id, *generation)
            && directory.join(snapshot_name(*generation)).is_file()
        {
            published += 1;
            if published == RETAINED_GENERATIONS {
                cutoff = Some(*generation);
                break;
            }
        }
    }
    let Some(cutoff) = cutoff else {
        return Ok(());
    };
    for (generation, path) in candidates {
        if generation > 0 && generation < cutoff {
            fs::remove_file(path)?;
        }
    }
    // Include orphaned snapshots from a crash before manifest publication or
    // during a previous cleanup, but never touch newer unpublished snapshots.
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

fn publish_json<T: Serialize>(directory: &Path, filename: &str, value: &T) -> io::Result<()> {
    let mut data = serde_json::to_vec(value).map_err(io::Error::other)?;
    data.push(b'\n');
    let temporary = directory.join(format!("{filename}.tmp"));
    let final_path = directory.join(filename);
    let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
    let result = (|| {
        file.write_all(&data)?;
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
    serde_json::from_slice(&fs::read(path)?).map_err(io::Error::other)
}

fn now_unix_ms() -> io::Result<u64> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).map_err(io::Error::other)?;
    u64::try_from(elapsed.as_millis()).map_err(io::Error::other)
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
