//! Disk identity and publication primitives for incremental chunk storage.
//!
//! This module deliberately contains no runtime residency policy. The save
//! catalog owns publication/recovery, while `DiskChunk` owns voxel encoding.
//! Keeping coordinate-to-path mapping and generation publication here gives
//! segmented storage one canonical filesystem boundary without teaching
//! streaming or worldgen about files.

use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use bevy::prelude::IVec3;

use crate::voxel::chunk_disk::DiskChunk;

const CHUNK_DIRECTORY: &str = "chunks";
const CHUNK_FILE_EXTENSION: &str = "json";
const GENERATION_DIRECTORY_PREFIX: &str = "generation-";
const STAGING_DIRECTORY_PREFIX: &str = ".generation-";
const STAGING_DIRECTORY_SUFFIX: &str = ".tmp";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ChunkDiskIdentity(IVec3);

impl ChunkDiskIdentity {
    pub(crate) fn new(coord: IVec3) -> io::Result<Self> {
        if coord.y < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "chunk disk identity cannot use a negative Y coordinate",
            ));
        }
        Ok(Self(coord))
    }

    /// Build the canonical identity directly from the portable `DiskChunk`
    /// coordinate representation. Keeping this conversion at the storage
    /// boundary prevents catalog code from reimplementing coordinate rules.
    pub(crate) fn from_disk_coord(coord: [i32; 3]) -> io::Result<Self> {
        Self::new(IVec3::new(coord[0], coord[1], coord[2]))
    }

    /// Resolve the storage identity of an encoded chunk without exposing its
    /// portable coordinate representation to catalog publication code.
    pub(crate) fn from_disk_chunk(chunk: &DiskChunk) -> io::Result<Self> {
        Self::from_disk_coord(chunk.coord)
    }

    pub(crate) fn coord(self) -> IVec3 {
        self.0
    }

    /// Stable relative path for one authoritative chunk inside a generation.
    /// Signed X/Z are explicit in the filename; Y was validated on creation.
    pub(crate) fn relative_path(self) -> PathBuf {
        Path::new(CHUNK_DIRECTORY).join(format!(
            "x{}_y{}_z{}.{}",
            self.0.x, self.0.y, self.0.z, CHUNK_FILE_EXTENSION
        ))
    }

    /// Generation-scoped path used by an immutable published save generation.
    pub(crate) fn generation_relative_path(self, generation: u64) -> PathBuf {
        generation_directory(generation).join(self.relative_path())
    }
}

/// Canonical directory containing all chunk blobs owned by one immutable save
/// generation. Catalog publication and pruning must use the same mapping.
pub(crate) fn generation_directory(generation: u64) -> PathBuf {
    PathBuf::from(format!("{GENERATION_DIRECTORY_PREFIX}{generation}"))
}

/// Private directory used while a generation's chunk set is being assembled.
pub(crate) fn generation_staging_directory(generation: u64) -> PathBuf {
    PathBuf::from(format!(
        "{STAGING_DIRECTORY_PREFIX}{generation}{STAGING_DIRECTORY_SUFFIX}"
    ))
}

/// Reject a path that already exists with the wrong filesystem type.
///
/// Publication and pruning use this before touching generation directories so
/// a symlink cannot redirect chunk I/O outside the locked world directory.
pub(crate) fn validate_generation_directory_slot(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "chunk generation path must be a real directory",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Assemble a complete immutable chunk generation privately and atomically
/// promote its directory identity. The catalog still owns the later manifest
/// commit, so a crash after this function leaves an unreferenced generation,
/// never a manifest pointing at a partially written chunk set.
pub(crate) fn publish_chunk_generation(
    world_directory: &Path,
    generation: u64,
    chunks: &[DiskChunk],
) -> io::Result<()> {
    let staging = world_directory.join(generation_staging_directory(generation));
    let published = world_directory.join(generation_directory(generation));
    validate_generation_directory_slot(&staging)?;
    validate_generation_directory_slot(&published)?;

    if published.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "chunk generation is already published",
        ));
    }
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }

    fs::create_dir(&staging)?;
    let chunks_directory = staging.join(CHUNK_DIRECTORY);
    fs::create_dir(&chunks_directory)?;

    let result = (|| {
        let mut identities = HashSet::with_capacity(chunks.len());
        for chunk in chunks {
            let identity = ChunkDiskIdentity::from_disk_chunk(chunk)?;
            if !identities.insert(identity) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "chunk generation contains a duplicate coordinate",
                ));
            }
            let path = staging.join(identity.relative_path());
            let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
            serde_json::to_writer(&mut file, chunk).map_err(io::Error::other)?;
            file.write_all(b"\n")?;
            file.sync_all()?;

            debug_assert_eq!(
                world_directory.join(identity.generation_relative_path(generation)),
                published.join(identity.relative_path())
            );
        }
        sync_directory(&chunks_directory)?;
        sync_directory(&staging)?;
        fs::rename(&staging, &published)?;
        sync_directory(world_directory)
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

/// Remove a generation directory only after verifying it is a real directory.
pub(crate) fn remove_chunk_generation(world_directory: &Path, generation: u64) -> io::Result<()> {
    let path = world_directory.join(generation_directory(generation));
    validate_generation_directory_slot(&path)?;
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn sync_directory(path: &Path) -> io::Result<()> {
    fs::File::open(path)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_round_trips_coordinate_and_uses_canonical_path() {
        let coord = IVec3::new(-12, 3, 45);
        let identity = ChunkDiskIdentity::new(coord).expect("valid chunk coordinate");
        assert_eq!(identity.coord(), coord);
        assert_eq!(identity.relative_path(), Path::new("chunks").join("x-12_y3_z45.json"));
    }

    #[test]
    fn disk_coordinate_uses_the_same_canonical_identity() {
        let identity = ChunkDiskIdentity::from_disk_coord([-12, 3, 45]).expect("portable disk coordinate must map to storage identity");
        assert_eq!(identity.coord(), IVec3::new(-12, 3, 45));
        assert_eq!(identity.relative_path(), Path::new("chunks").join("x-12_y3_z45.json"));
    }

    #[test]
    fn generation_path_preserves_canonical_chunk_identity() {
        let identity = ChunkDiskIdentity::new(IVec3::new(-12, 3, 45)).expect("valid chunk coordinate");
        assert_eq!(identity.generation_relative_path(7), Path::new("generation-7").join("chunks").join("x-12_y3_z45.json"));
    }

    #[test]
    fn generation_directory_is_shared_by_publication_and_pruning() {
        assert_eq!(generation_directory(7), Path::new("generation-7"));
    }

    #[test]
    fn staging_directory_cannot_be_mistaken_for_published_generation() {
        assert_eq!(generation_staging_directory(7), Path::new(".generation-7.tmp"));
        assert_ne!(generation_staging_directory(7), generation_directory(7));
    }

    #[test]
    fn identity_rejects_negative_vertical_coordinate() {
        let error = ChunkDiskIdentity::new(IVec3::new(0, -1, 0)).expect_err("negative chunk Y must never reach disk identity");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn disk_coordinate_rejects_negative_vertical_coordinate() {
        let error = ChunkDiskIdentity::from_disk_coord([0, -1, 0]).expect_err("portable disk coordinate must obey storage identity rules");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}