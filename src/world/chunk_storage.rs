//! Disk identity for the incremental chunk-storage boundary.
//!
//! This module deliberately contains no runtime residency policy. The save
//! catalog owns publication/recovery, while `DiskChunk` owns voxel encoding.
//! Keeping the coordinate-to-path mapping here gives segmented storage one
//! canonical identity without teaching streaming or worldgen about files.

use std::{
    io,
    path::{Path, PathBuf},
};

use bevy::prelude::IVec3;

use crate::voxel::chunk_disk::DiskChunk;

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

    /// Stable relative path for one authoritative chunk. Signed X/Z are kept
    /// explicit in the filename; Y is validated non-negative before this point.
    pub(crate) fn relative_path(self) -> PathBuf {
        Path::new(CHUNK_DIRECTORY).join(format!(
            "x{}_y{}_z{}.{}",
            self.0.x, self.0.y, self.0.z, CHUNK_FILE_EXTENSION
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_round_trips_coordinate_and_uses_canonical_path() {
        let coord = IVec3::new(-12, 3, 45);
        let identity = ChunkDiskIdentity::new(coord).expect("valid chunk coordinate");

        assert_eq!(identity.coord(), coord);
        assert_eq!(
            identity.relative_path(),
            Path::new("chunks").join("x-12_y3_z45.json")
        );
    }

    #[test]
    fn disk_coordinate_uses_the_same_canonical_identity() {
        let identity = ChunkDiskIdentity::from_disk_coord([-12, 3, 45])
            .expect("portable disk coordinate must map to storage identity");

        assert_eq!(identity.coord(), IVec3::new(-12, 3, 45));
        assert_eq!(
            identity.relative_path(),
            Path::new("chunks").join("x-12_y3_z45.json")
        );
    }

    #[test]
    fn identity_rejects_negative_vertical_coordinate() {
        let error = ChunkDiskIdentity::new(IVec3::new(0, -1, 0))
            .expect_err("negative chunk Y must never reach disk identity");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn disk_coordinate_rejects_negative_vertical_coordinate() {
        let error = ChunkDiskIdentity::from_disk_coord([0, -1, 0])
            .expect_err("portable disk coordinate must obey storage identity rules");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
