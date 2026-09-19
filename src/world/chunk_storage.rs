//! Disk identity for the incremental chunk-storage boundary.
//!
//! This module deliberately contains no runtime residency policy. The save
//! catalog owns publication/recovery, while `DiskChunk` owns voxel encoding.
//! Keeping the coordinate-to-path mapping here gives segmented storage one
//! canonical identity without teaching streaming or worldgen about files.

use std::{io, path::{Path, PathBuf}};

use bevy::prelude::IVec3;

const CHUNK_DIRECTORY: &str = "chunks";
const CHUNK_FILE_EXTENSION: &str = "json";

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

    pub(crate) fn coord(self) -> IVec3 {
        self.0
    }

    /// Stable relative path for one authoritative chunk. Signed X/Z are kept
    /// explicit in the filename; Y is validated non-negative before this point.
    pub(crate) fn relative_path(self) -> PathBuf {
        Path::new(CHUNK_DIRECTORY).join(format!(
            "x{}_y{}_z{}.{}",
            self.0.x, self.0.y, self.0.z, CHUNK_FILE_EXTENSION
        ))
    }
}
