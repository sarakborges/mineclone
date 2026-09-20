mod chunks;
mod locking;
mod snapshot;
mod storage;
mod validation;

use std::{
    fs,
    io,
    path::{Path, PathBuf},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::log::warn;
use crate::{
    content::{block::BlockRegistry, fluid::FluidRegistry},
    voxel::world::VoxelWorld,
};

use self::{
    chunks::SavedChunkCatalog,
    locking::{
        ReadLease, acquire_world_directory_lock, remove_world_directory_lock_file, world_lock,
    },
    snapshot::{
        is_supported_save_format, StoredWorldSnapshot, WorldManifest,
        LEGACY_SAVE_FORMAT_VERSION, SAVE_FORMAT_VERSION,
    },
    storage::{
        highest_generation, manifest_name, manifest_paths, open_snapshot_file, publish_json,
        read_json, read_json_file, snapshot_generation, snapshot_name,
    },
};
pub(crate) use self::{
    locking::WorldDirectoryLock,
    snapshot::{SavedPlayer, SnapshotSource, WorldSnapshot},
    validation::{PruneRegistries, SaveRegistries},
};

use super::{
    chunk_storage::{
        generation_chunks_published, generation_storage_slot_exists, load_generation_chunks,
        publish_generation_chunks, remove_generation_chunks,
    },
    new_world::{
        WorldgenVersion, biome_size_multiplier_tenths, is_valid_biome_size_multiplier,
    },
    world_names::{WORLDS_DIRECTORY, available_world_name, validate_world_name},
};

const RETAINED_GENERATIONS: usize = 4;

#[derive(Clone, Debug)]
pub(crate) struct WorldSummary {
    pub(crate) id: String,
    pub(crate) last_saved_unix_ms: u64,
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

pub(crate) fn save_world(snapshot: &WorldSnapshot, registries: SaveRegistries<'_>) -> io::Result<u64> {
    registries.validate_playable(snapshot)?; save_world_owned(snapshot, registries.owned_for_pruning())
}
pub(crate) fn save_world_owned(
    snapshot: &WorldSnapshot,
    registries: PruneRegistries,
) -> io::Result<u64> {
    validate_world_name(&snapshot.id)?;
    let gate = world_lock(&snapshot.id)?;
    let lock = gate.lock_write()?;
    if snapshot.format_version != SAVE_FORMAT_VERSION {
        return Err(invalid_data("unsupported runtime snapshot format"));
    }
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
        || !is_supported_save_format(initial.format_version)
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
    if let Err(error) = publish_json(&directory, &snapshot_file, &snapshot.disk_v2()) {
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
    let entries = match fs::read_dir(WORLDS_DIRECTORY) { Ok(entries) => entries, Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()), Err(error) => return Err(error) };
    let mut worlds = Vec::new();
    for entry in entries {
        let entry = entry?; if !entry.file_type()?.is_dir() { continue; }
        let Some(id) = entry.file_name().to_str().map(str::to_owned) else { continue; };
        if validate_world_name(&id).is_err() { continue; }
        let gate = world_lock(&id)?; let _lock = gate.lock_write()?;
        if let Ok(manifest) = latest_complete_manifest(&entry.path(), &id) { worlds.push(WorldSummary { id, last_saved_unix_ms: manifest.last_saved_unix_ms }); }
    }
    worlds.sort_unstable_by(|a,b| b.last_saved_unix_ms.cmp(&a.last_saved_unix_ms).then_with(|| a.id.cmp(&b.id))); Ok(worlds)
}
pub(crate) fn list_verified_worlds(registries: &PruneRegistries) -> io::Result<Vec<WorldSummary>> {
    let candidates = list_worlds()?; let mut verified = Vec::with_capacity(candidates.len());
    for candidate in candidates { match newest_restorable_timestamp(&candidate.id, registries) {
        Ok(timestamp) => verified.push(WorldSummary { id: candidate.id, last_saved_unix_ms: timestamp }),
        Err(error) => warn!("World {} has no verified snapshot: {error}", candidate.id),
    }}
    verified.sort_unstable_by(|a,b| b.last_saved_unix_ms.cmp(&a.last_saved_unix_ms).then_with(|| a.id.cmp(&b.id))); Ok(verified)
}
struct Candidate { generation: u64, manifest: WorldManifest }
struct SnapshotCandidates { directory: PathBuf, candidates: Vec<Candidate>, _lease: ReadLease }
fn snapshot_candidates(id: &str) -> io::Result<SnapshotCandidates> {
    validate_world_name(id)?; let gate = world_lock(id)?;
    let (directory, candidates, lease) = {
        let _lock = gate.lock_write()?;
        let directory = Path::new(WORLDS_DIRECTORY).join(id);
        if !fs::symlink_metadata(&directory)?.file_type().is_dir() { return Err(invalid_data("world directory cannot be a symbolic link")); }
        let mut paths = manifest_paths(&directory)?; paths.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
        let mut candidates = Vec::with_capacity(paths.len());
        for (generation,path) in paths { let Ok(manifest) = read_json::<WorldManifest>(&path) else { continue; }; if valid_manifest(&manifest,id,generation) { candidates.push(Candidate { generation, manifest }); } }
        let lease = gate.pin_read(); (directory,candidates,lease)
    }; Ok(SnapshotCandidates { directory,candidates,_lease:lease })
}
fn newest_restorable_timestamp(
    id: &str,
    registries: &PruneRegistries,
) -> io::Result<u64> {
    let pinned = snapshot_candidates(id)?;
    for candidate in &pinned.candidates {
        let loaded = load_snapshot(
            &pinned.directory,
            id,
            &candidate.manifest,
            &registries.blocks,
            &registries.fluids,
            |snapshot| registries.validate_playable(snapshot),
        );
        match loaded {
            Ok(_) => return Ok(candidate.manifest.last_saved_unix_ms),
            Err(error) => warn!(
                "Skipping damaged save for world {id}, generation {}: {error}",
                candidate.generation
            ),
        }
    }
    Err(invalid_data(format!("world {id} has no restorable save")))
}

pub(crate) fn load_world(
    id: &str,
    registries: SaveRegistries<'_>,
) -> io::Result<(WorldSnapshot, VoxelWorld, WorldDirectoryLock)> {
    validate_world_name(id)?;
    let directory = Path::new(WORLDS_DIRECTORY).join(id);
    if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
        return Err(invalid_data("world directory cannot be a symbolic link"));
    }

    let session_lock = acquire_world_directory_lock(&directory)?;
    let pinned = snapshot_candidates(id)?;
    let mut last_error = None;
    for candidate in &pinned.candidates {
        let loaded = load_snapshot(
            &pinned.directory,
            id,
            &candidate.manifest,
            registries.blocks,
            registries.fluids,
            |snapshot| registries.validate_playable(snapshot),
        );
        match loaded {
            Ok((snapshot, world)) => return Ok((snapshot, world, session_lock)),
            Err(error) => {
                warn!(
                    "Skipping damaged save for world {id}, generation {}: {error}",
                    candidate.generation
                );
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
    let file = open_snapshot_file(directory, manifest)?;
    decode_snapshot(directory, file, id, manifest, blocks, fluids, validate)
}

fn decode_snapshot(
    directory: &Path,
    file: fs::File,
    id: &str,
    manifest: &WorldManifest,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    validate: impl FnOnce(&WorldSnapshot) -> io::Result<()>,
) -> io::Result<(WorldSnapshot, VoxelWorld)> {
    let stored: StoredWorldSnapshot = read_json_file(file)?;
    if !is_supported_save_format(stored.format_version)
        || stored.format_version != manifest.format_version
    {
        return Err(invalid_data("snapshot and manifest format do not match"));
    }

    manifest
        .worldgen_version
        .validate_matches(stored.worldgen_version)?;
    if stored.id != id
        || stored.seed != manifest.seed
        || stored.dimension_id != manifest.dimension_id
        || biome_size_multiplier_tenths(stored.biome_size_multiplier)
            != biome_size_multiplier_tenths(manifest.biome_size_multiplier)
        || !is_valid_biome_size_multiplier(stored.biome_size_multiplier)
        || stored.ticks_per_second != manifest.ticks_per_second
        || stored.ticks_per_second == 0
        || stored.day == 0
        || stored
            .player
            .as_ref()
            .is_some_and(|player| player.position.iter().any(|coord| !coord.is_finite()))
    {
        return Err(invalid_data("snapshot metadata or player state is invalid"));
    }

    let stored_format = stored.format_version;
    let (snapshot, inline_chunks) = stored.into_runtime()?;
    validate(&snapshot)?;

    let chunks = match stored_format {
        LEGACY_SAVE_FORMAT_VERSION => inline_chunks
            .ok_or_else(|| invalid_data("format v1 snapshot is missing inline chunks"))?,
        SAVE_FORMAT_VERSION => {
            if inline_chunks.is_some() {
                return Err(invalid_data(
                    "format v2 snapshot has conflicting inline chunk state",
                ));
            }
            SavedChunkCatalog::from_disk_chunks(load_generation_chunks(
                directory,
                manifest.generation,
            )?)
        }
        _ => return Err(invalid_data("unsupported snapshot format")),
    };

    let world = chunks.into_world(blocks, fluids)?;
    Ok((snapshot, world))
}

fn valid_manifest(manifest: &WorldManifest, id: &str, generation: u64) -> bool {
    is_supported_save_format(manifest.format_version)
        && manifest.id == id
        && manifest.generation == generation
        && manifest.worldgen_version.validate().is_ok()
        && is_valid_biome_size_multiplier(manifest.biome_size_multiplier)
        && manifest.ticks_per_second > 0
        && generation > 0
        && manifest.snapshot_file.as_deref() == Some(snapshot_name(generation).as_str())
}

fn manifest_payload_published(
    directory: &Path,
    manifest: &WorldManifest,
) -> io::Result<bool> {
    if !directory.join(snapshot_name(manifest.generation)).is_file() {
        return Ok(false);
    }
    match manifest.format_version {
        LEGACY_SAVE_FORMAT_VERSION => Ok(true),
        SAVE_FORMAT_VERSION => generation_chunks_published(directory, manifest.generation),
        _ => Ok(false),
    }
}

fn latest_complete_manifest(directory: &Path, id: &str) -> io::Result<WorldManifest> {
    let mut candidates = manifest_paths(directory)?;
    candidates.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
    for (generation, path) in candidates {
        let Ok(manifest) = read_json::<WorldManifest>(&path) else {
            continue;
        };
        if !valid_manifest(&manifest, id, generation) {
            continue;
        }
        match manifest_payload_published(directory, &manifest) {
            Ok(true) => return Ok(manifest),
            Ok(false) => {}
            Err(error) => warn!(
                "Skipping incomplete save for world {id}, generation {generation}: {error}"
            ),
        }
    }
    Err(invalid_data(format!(
        "world {id} has no published complete save"
    )))
}

fn prune_old_generations(
    directory: &Path,
    id: &str,
    registries: &PruneRegistries,
) -> io::Result<()> {
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
            Err(error) => warn!(
                "Preserving older saves because world {id} generation {generation} is not restorable: {error}"
            ),
        }
    }

    let Some(cutoff) = cutoff else {
        return Ok(());
    };
    let gate = world_lock(id)?;
    let _write = gate.lock_after_readers()?;
    if !fs::symlink_metadata(directory)?.file_type().is_dir() {
        return Err(invalid_data("world directory cannot be a symbolic link"));
    }

    let stale_generations = candidates
        .iter()
        .filter_map(|(generation, _)| {
            (*generation > 0 && *generation < cutoff).then_some(*generation)
        })
        .collect::<Vec<_>>();

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
        let Some(generation) = snapshot_generation(&name) else {
            continue;
        };
        if generation > 0 && generation < cutoff {
            fs::remove_file(entry.path())?;
        }
    }

    for generation in stale_generations {
        remove_generation_chunks(directory, generation)?;
    }

    Ok(())
}
fn now_unix_ms() -> io::Result<u64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?;
    u64::try_from(elapsed.as_millis()).map_err(io::Error::other)
}

fn invalid_data(message:impl Into<String>)->io::Error{io::Error::new(io::ErrorKind::InvalidData,message.into())}
