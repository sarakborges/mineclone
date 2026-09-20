use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

use bevy::log::warn;

use crate::{
    content::{block::BlockRegistry, fluid::FluidRegistry},
    voxel::world::VoxelWorld,
};

use super::{
    chunks::SavedChunkCatalog,
    invalid_data,
    locking::{ReadLease, WorldDirectoryLock, acquire_world_directory_lock, world_lock},
    snapshot::{
        LEGACY_SAVE_FORMAT_VERSION, SAVE_FORMAT_VERSION, StoredWorldSnapshot, WorldManifest,
        WorldSnapshot, is_supported_save_format,
    },
    storage::{
        manifest_paths, open_snapshot_file, read_json, read_json_file, snapshot_generation,
        snapshot_name,
    },
    validation::{PruneRegistries, SaveRegistries},
};
use crate::world::{
    chunk_storage::{
        generation_chunks_published, load_generation_chunks, remove_generation_chunks,
    },
    new_world::{biome_size_multiplier_tenths, is_valid_biome_size_multiplier},
    world_names::{WORLDS_DIRECTORY, validate_world_name},
};

pub(super) const RETAINED_GENERATIONS: usize = 4;

struct Candidate {
    generation: u64,
    manifest: WorldManifest,
}

struct SnapshotCandidates {
    directory: PathBuf,
    candidates: Vec<Candidate>,
    _lease: ReadLease,
}

fn snapshot_candidates(id: &str) -> io::Result<SnapshotCandidates> {
    validate_world_name(id)?;
    let gate = world_lock(id)?;
    let (directory, candidates, lease) = {
        let _lock = gate.lock_write()?;
        let directory = Path::new(WORLDS_DIRECTORY).join(id);
        if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
            return Err(invalid_data("world directory cannot be a symbolic link"));
        }

        let mut paths = manifest_paths(&directory)?;
        paths.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
        let mut candidates = Vec::with_capacity(paths.len());
        for (generation, path) in paths {
            let Ok(manifest) = read_json::<WorldManifest>(&path) else {
                continue;
            };
            if valid_manifest(&manifest, id, generation) {
                candidates.push(Candidate {
                    generation,
                    manifest,
                });
            }
        }

        let lease = gate.pin_read();
        (directory, candidates, lease)
    };

    Ok(SnapshotCandidates {
        directory,
        candidates,
        _lease: lease,
    })
}

pub(super) fn newest_restorable_timestamp(
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

pub(super) fn latest_complete_manifest(
    directory: &Path,
    id: &str,
) -> io::Result<WorldManifest> {
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

pub(super) fn prune_old_generations(
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
