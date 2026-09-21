mod chunks;
mod generations;
mod locking;
mod snapshot;
mod storage;
mod validation;

use std::{
    fs,
    io,
    path::Path,
    process::Command,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::log::warn;
use self::{
    generations::{
        RETAINED_GENERATIONS, latest_complete_manifest, newest_restorable_summary,
        prune_old_generations,
    },
    locking::{
        acquire_world_directory_lock, remove_world_directory_lock_file, world_lock,
    },
    snapshot::{WorldManifest, SAVE_FORMAT_VERSION},
    storage::{highest_generation, manifest_name, publish_json, read_json, snapshot_name},
};
pub(crate) use self::{
    generations::load_world,
    locking::WorldDirectoryLock,
    snapshot::{SavedPlayer, SnapshotSource, WorldSnapshot},
    validation::{PruneRegistries, SaveRegistries},
};

use super::{
    chunk_storage::{
        generation_storage_slot_exists, publish_generation_chunks, remove_generation_chunks,
    },
    new_world::{
        WorldgenVersion, biome_size_multiplier_tenths, is_valid_biome_size_multiplier,
    },
    world_names::{WORLDS_DIRECTORY, available_world_name, validate_world_name},
};

#[derive(Clone, Debug)]
pub(crate) struct WorldSummary {
    pub(crate) id: String,
    pub(crate) last_saved_unix_ms: u64,
    pub(crate) seed: u64,
    pub(crate) day: u64,
    pub(crate) dimension_id: String,
    pub(crate) player_position: Option<[f32; 3]>,
    pub(crate) biome_id: Option<String>,
}

pub(crate) fn open_worlds_directory() -> io::Result<()> {
    fs::create_dir_all(WORLDS_DIRECTORY)?;
    let directory = std::env::current_dir()?.join(WORLDS_DIRECTORY);
    open_directory(&directory)
}

#[cfg(target_os = "windows")]
fn open_directory(directory: &Path) -> io::Result<()> {
    Command::new("explorer.exe").arg(directory).spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_directory(directory: &Path) -> io::Result<()> {
    Command::new("open").arg(directory).spawn()?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_directory(directory: &Path) -> io::Result<()> {
    Command::new("xdg-open").arg(directory).spawn()?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn open_directory(_directory: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "opening the saves directory is unsupported on this platform",
    ))
}

pub(crate) fn create_new_world(requested_name: &str, seed: u64, dimension_id: &str, biome_size_multiplier: f32, ticks_per_second: u32) -> io::Result<(String, WorldDirectoryLock)> {
    if ticks_per_second == 0 || dimension_id.is_empty() { return Err(io::Error::new(io::ErrorKind::InvalidInput, "World seed metadata must include a dimension and a positive tick rate")); }
    if !is_valid_biome_size_multiplier(biome_size_multiplier) { return Err(io::Error::new(io::ErrorKind::InvalidInput, "Biome size multiplier must be between 0.5 and 5.0 in 0.1 increments")); }
    let created_at = now_unix_ms()?;
    let root = Path::new(WORLDS_DIRECTORY); fs::create_dir_all(root)?;
    let mut candidate = available_world_name(requested_name)?;
    loop {
        validate_world_name(&candidate)?;
        let directory = root.join(&candidate);
        match fs::create_dir(&directory) {
            Ok(()) => {
                let session_lock = match acquire_world_directory_lock(&directory) {
                    Ok(lock) => lock,
                    Err(error) => { let _ = remove_world_directory_lock_file(&directory); let _ = fs::remove_dir(&directory); return Err(error); }
                };
                let manifest = WorldManifest {
                    format_version: SAVE_FORMAT_VERSION, id: candidate.clone(), seed, dimension_id: dimension_id.to_owned(),
                    worldgen_version: WorldgenVersion::current(), biome_size_multiplier, ticks_per_second,
                    last_saved_unix_ms: created_at, generation: 0, snapshot_file: None,
                };
                if let Err(error) = publish_json(&directory, &manifest_name(0), &manifest) {
                    drop(session_lock); let _ = fs::remove_file(directory.join(format!("{}.tmp", manifest_name(0))));
                    let _ = remove_world_directory_lock_file(&directory); let _ = fs::remove_dir(&directory); return Err(error);
                }
                return Ok((candidate, session_lock));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => candidate = available_world_name(requested_name)?,
            Err(error) => return Err(error),
        }
    }
}

pub(crate) fn save_world(
    snapshot: &WorldSnapshot,
    registries: SaveRegistries<'_>,
) -> io::Result<u64> {
    save_world_owned(snapshot, registries.owned_for_pruning())
}
pub(crate) fn save_world_owned(
    snapshot: &WorldSnapshot,
    registries: PruneRegistries,
) -> io::Result<u64> {
    validate_world_name(&snapshot.id)?;
    let gate = world_lock(&snapshot.id)?;
    let lock = gate.lock_write()?;
    registries.validate_playable(snapshot)?;

    let directory = Path::new(WORLDS_DIRECTORY).join(&snapshot.id);
    if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
        return Err(invalid_data("world directory cannot be a symbolic link"));
    }

    let initial: WorldManifest = read_json(&directory.join(manifest_name(0)))?;
    initial
        .worldgen_version
        .validate_matches(snapshot.worldgen_version)?;
    if initial.id != snapshot.id
        || initial.seed != snapshot.seed
        || initial.dimension_id != snapshot.dimension_id
        || biome_size_multiplier_tenths(initial.biome_size_multiplier)
            != biome_size_multiplier_tenths(snapshot.biome_size_multiplier)
        || initial.format_version != SAVE_FORMAT_VERSION
        || initial.generation != 0
        || initial.snapshot_file.is_some()
    {
        return Err(invalid_data(
            "snapshot does not match reserved world identity",
        ));
    }

    let mut next = highest_generation(&directory)?
        .checked_add(1)
        .ok_or_else(|| io::Error::other("world save generation counter exhausted"))?;
    loop {
        let occupied = directory.join(snapshot_name(next)).exists()
            || directory.join(manifest_name(next)).exists()
            || directory
                .join(format!("{}.tmp", snapshot_name(next)))
                .exists()
            || directory
                .join(format!("{}.tmp", manifest_name(next)))
                .exists()
            || generation_storage_slot_exists(&directory, next)?;
        if !occupied {
            break;
        }
        next = next
            .checked_add(1)
            .ok_or_else(|| io::Error::other("save counter exhausted"))?;
    }

    let saved_at = now_unix_ms()?;

    if let Err(error) =
        publish_generation_chunks(&directory, next, snapshot.chunks.disk_chunks())
    {
        cleanup_unpublished_generation(&directory, next, None);
        return Err(error);
    }

    let snapshot_file = snapshot_name(next);
    if let Err(error) = publish_json(&directory, &snapshot_file, &snapshot.disk_snapshot()) {
        cleanup_unpublished_generation(&directory, next, Some(&snapshot_file));
        return Err(error);
    }

    let manifest = WorldManifest {
        format_version: SAVE_FORMAT_VERSION,
        id: snapshot.id.clone(),
        seed: snapshot.seed,
        dimension_id: snapshot.dimension_id.clone(),
        worldgen_version: snapshot.worldgen_version,
        biome_size_multiplier: snapshot.biome_size_multiplier,
        ticks_per_second: snapshot.ticks_per_second,
        last_saved_unix_ms: saved_at,
        generation: next,
        snapshot_file: Some(snapshot_file.clone()),
    };
    if let Err(error) = publish_json(&directory, &manifest_name(next), &manifest) {
        cleanup_unpublished_generation(&directory, next, Some(&snapshot_file));
        return Err(error);
    }

    drop(lock);
    if next > RETAINED_GENERATIONS as u64 {
        schedule_backup_prune(snapshot.id.clone(), registries);
    }
    Ok(saved_at)
}

fn cleanup_unpublished_generation(
    directory: &Path,
    generation: u64,
    snapshot_file: Option<&str>,
) {
    if let Some(snapshot_file) = snapshot_file
        && let Err(error) = fs::remove_file(directory.join(snapshot_file))
        && error.kind() != io::ErrorKind::NotFound
    {
        warn!("Could not remove unpublished world snapshot: {error}");
    }
    if let Err(error) = remove_generation_chunks(directory, generation) {
        warn!("Could not remove unpublished chunk generation {generation}: {error}");
    }
}

fn schedule_backup_prune(id: String, owned: PruneRegistries) {
    let gate = match world_lock(&id) {
        Ok(gate) => gate,
        Err(error) => {
            warn!("World {id} was saved, but its cleanup lock is unavailable: {error}");
            return;
        }
    };
    let Some(prune_lease) = gate.try_begin_prune() else {
        return;
    };

    let spawned = thread::Builder::new()
        .name("asteria-save-prune".to_owned())
        .spawn(move || {
            let _prune_lease = prune_lease;
            let directory = Path::new(WORLDS_DIRECTORY).join(&id);
            if let Err(error) = prune_old_generations(&directory, &id, &owned) {
                warn!("World {id} was saved, but background backup cleanup failed: {error}");
            }
        });
    if let Err(error) = spawned {
        warn!("World was saved, but backup cleanup thread could not start: {error}");
    }
}
pub(crate) fn delete_world(id: &str) -> io::Result<()> {
    validate_world_name(id)?; let gate = world_lock(id)?;
    let _lock = gate.lock_write()?;
    let directory = Path::new(WORLDS_DIRECTORY).join(id); let metadata = fs::symlink_metadata(&directory)?;
    if !metadata.file_type().is_dir() { return Err(invalid_data("world directory cannot be a symbolic link")); }
    let session_lock = acquire_world_directory_lock(&directory)?; fs::remove_dir_all(&directory)?; drop(session_lock); Ok(())
}
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
        let gate = world_lock(&id)?;
        let _lock = gate.lock_write()?;
        if let Ok(manifest) = latest_complete_manifest(&entry.path(), &id) {
            worlds.push(WorldSummary {
                id,
                last_saved_unix_ms: manifest.last_saved_unix_ms,
                seed: manifest.seed,
                day: 0,
                dimension_id: manifest.dimension_id,
                player_position: None,
                biome_id: None,
            });
        }
    }
    worlds.sort_unstable_by(|a, b| {
        b.last_saved_unix_ms
            .cmp(&a.last_saved_unix_ms)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(worlds)
}

pub(crate) fn list_verified_worlds(
    registries: &PruneRegistries,
) -> io::Result<Vec<WorldSummary>> {
    let candidates = list_worlds()?;
    let mut verified = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        match newest_restorable_summary(&candidate.id, registries) {
            Ok(summary) => verified.push(summary),
            Err(error) => warn!("World {} has no verified snapshot: {error}", candidate.id),
        }
    }
    verified.sort_unstable_by(|a, b| {
        b.last_saved_unix_ms
            .cmp(&a.last_saved_unix_ms)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(verified)
}
fn now_unix_ms() -> io::Result<u64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?;
    u64::try_from(elapsed.as_millis()).map_err(io::Error::other)
}

fn invalid_data(message:impl Into<String>)->io::Error{io::Error::new(io::ErrorKind::InvalidData,message.into())}
