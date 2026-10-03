use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::{
    content::{
        block::BlockRegistry, fluid::FluidRegistry, layer::LayerRegistry, object::ObjectRegistry,
    },
    voxel::world::VoxelWorld,
    world::{
        chunk_storage::{
            generation_chunks_published, generation_storage_slot_exists, load_generation_world,
            publish_generation_world_chunks, remove_generation_chunks,
        },
        storage_durability::sync_directory,
    },
};

const DIMENSIONS_DIRECTORY: &str = "dimensions";

fn encoded_dimension_id(id: &str) -> String {
    let mut encoded = String::with_capacity(id.len() * 2);
    for byte in id.as_bytes() {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn dimensions_root(world_directory: &Path) -> PathBuf {
    world_directory.join(DIMENSIONS_DIRECTORY)
}

pub(super) fn dimension_directory(world_directory: &Path, dimension_id: &str) -> PathBuf {
    dimensions_root(world_directory).join(encoded_dimension_id(dimension_id))
}

fn ensure_real_directory(path: &Path, label: &str) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            Ok(())
        }
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} must be a real directory: {}", path.display()),
        )),
        Err(error) => Err(error),
    }
}

fn ensure_dimensions_root(world_directory: &Path) -> io::Result<PathBuf> {
    let root = dimensions_root(world_directory);
    match fs::create_dir(&root) {
        Ok(()) => {
            sync_directory(world_directory)?;
            Ok(root)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            ensure_real_directory(&root, "dimension storage root")?;
            Ok(root)
        }
        Err(error) => Err(error),
    }
}

fn ensure_dimension_directory(world_directory: &Path, dimension_id: &str) -> io::Result<PathBuf> {
    let root = ensure_dimensions_root(world_directory)?;
    let directory = root.join(encoded_dimension_id(dimension_id));
    match fs::create_dir(&directory) {
        Ok(()) => {
            sync_directory(&root)?;
            Ok(directory)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            ensure_real_directory(&directory, "dimension storage directory")?;
            Ok(directory)
        }
        Err(error) => Err(error),
    }
}

pub(super) fn publish_generation_worlds<'a, I>(
    world_directory: &Path,
    generation: u64,
    worlds: I,
    fluids: &FluidRegistry,
) -> io::Result<()>
where
    I: IntoIterator<Item = (&'a str, &'a VoxelWorld)>,
{
    let mut seen = std::collections::HashSet::new();
    for (dimension_id, world) in worlds {
        if !seen.insert(dimension_id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("duplicate persisted dimension: {dimension_id}"),
            ));
        }
        let directory = ensure_dimension_directory(world_directory, dimension_id)?;
        publish_generation_world_chunks(&directory, generation, world, fluids)?;
    }
    if seen.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "world save must contain at least one dimension",
        ));
    }
    Ok(())
}

pub(super) fn generation_worlds_published(
    world_directory: &Path,
    generation: u64,
    dimension_ids: &[String],
) -> io::Result<bool> {
    if dimension_ids.is_empty() {
        return Ok(false);
    }
    for dimension_id in dimension_ids {
        let directory = dimension_directory(world_directory, dimension_id);
        match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            }
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "dimension storage directory must be a real directory: {}",
                        directory.display()
                    ),
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error),
        }
        if !generation_chunks_published(&directory, generation)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn generation_world_storage_slot_exists(
    world_directory: &Path,
    generation: u64,
) -> io::Result<bool> {
    let root = dimensions_root(world_directory);
    match fs::read_dir(&root) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                if !entry.file_type()?.is_dir() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "dimension storage entry must be a directory: {}",
                            entry.path().display()
                        ),
                    ));
                }
                if generation_storage_slot_exists(&entry.path(), generation)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub(super) fn load_dimension_world(
    world_directory: &Path,
    generation: u64,
    dimension_id: &str,
    blocks: &BlockRegistry,
    layers: &LayerRegistry,
    objects: &ObjectRegistry,
    fluids: &FluidRegistry,
) -> io::Result<VoxelWorld> {
    let directory = dimension_directory(world_directory, dimension_id);
    ensure_real_directory(&directory, "dimension storage directory")?;
    load_generation_world(&directory, generation, blocks, layers, objects, fluids)
}

pub(super) fn remove_generation_worlds(world_directory: &Path, generation: u64) -> io::Result<()> {
    let root = dimensions_root(world_directory);
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "dimension storage entry must be a directory: {}",
                    entry.path().display()
                ),
            ));
        }
        remove_generation_chunks(&entry.path(), generation)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_directory_encoding_is_path_safe_and_stable() {
        assert_eq!(
            encoded_dimension_id("asteria:overworld"),
            "617374657269613a6f766572776f726c64"
        );
        assert_eq!(
            encoded_dimension_id("asteria:umbral"),
            "617374657269613a756d6272616c"
        );
    }
}
