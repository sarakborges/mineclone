use std::{io, path::Path};

use serde::{Deserialize, Serialize};

use crate::voxel::chunk_disk::DiskChunk;

use super::chunk_storage::{
    generation_slot_occupied, publish_generation_chunks, read_generation_chunks,
    remove_generation_chunks,
};

/// Declares which persisted artifact is authoritative for a save generation's chunks.
///
/// The default is intentionally the legacy snapshot representation so manifests written
/// before external chunk storage existed keep their original load semantics.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ChunkStorageKind {
    #[default]
    Snapshot,
    GenerationDirectory,
}

impl ChunkStorageKind {
    /// Reports whether this ownership mode already occupies the generation slot.
    /// Snapshot-owned generations have no independent chunk-storage slot.
    pub(crate) fn generation_slot_occupied(
        self,
        world_directory: &Path,
        generation: u64,
    ) -> io::Result<bool> {
        match self {
            Self::Snapshot => Ok(false),
            Self::GenerationDirectory => generation_slot_occupied(world_directory, generation),
        }
    }

    /// Publishes chunks for this ownership mode and returns the representation that must
    /// remain in the snapshot. External generations deliberately return an empty vector:
    /// once their directory is published, serializing the same chunks into the snapshot
    /// would create two authoritative persisted owners.
    pub(crate) fn publish_chunks(
        self,
        world_directory: &Path,
        generation: u64,
        chunks: &[DiskChunk],
    ) -> io::Result<Vec<DiskChunk>> {
        match self {
            Self::Snapshot => Ok(chunks.to_vec()),
            Self::GenerationDirectory => {
                publish_generation_chunks(world_directory, generation, chunks)?;
                Ok(Vec::new())
            }
        }
    }

    /// Resolves the one authoritative persisted chunk representation for a generation.
    ///
    /// External-storage generations must not also carry snapshot chunks: accepting both
    /// would make recovery semantics depend on an arbitrary precedence rule and violate
    /// the save architecture's single-owner invariant.
    pub(crate) fn load_chunks(
        self,
        world_directory: &Path,
        generation: u64,
        snapshot_chunks: Vec<DiskChunk>,
    ) -> io::Result<Vec<DiskChunk>> {
        match self {
            Self::Snapshot => Ok(snapshot_chunks),
            Self::GenerationDirectory => {
                if !snapshot_chunks.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "save generation has both snapshot and external chunk owners",
                    ));
                }
                read_generation_chunks(world_directory, generation)
            }
        }
    }

    /// Removes storage owned by this mode. This is used both when publication of the
    /// generation's commit marker fails and when an old generation is pruned.
    pub(crate) fn remove_chunks(
        self,
        world_directory: &Path,
        generation: u64,
    ) -> io::Result<()> {
        match self {
            Self::Snapshot => Ok(()),
            Self::GenerationDirectory => remove_generation_chunks(world_directory, generation),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_manifest_field_defaults_to_legacy_snapshot_storage() {
        #[derive(Deserialize)]
        struct Fixture {
            #[serde(default)]
            chunk_storage: ChunkStorageKind,
        }

        let fixture: Fixture = serde_json::from_str("{}").expect("legacy fixture must decode");
        assert_eq!(fixture.chunk_storage, ChunkStorageKind::Snapshot);
    }

    #[test]
    fn generation_directory_has_stable_serialized_name() {
        assert_eq!(
            serde_json::to_string(&ChunkStorageKind::GenerationDirectory)
                .expect("storage kind must serialize"),
            "\"generation_directory\""
        );
    }

    #[test]
    fn legacy_snapshot_storage_keeps_snapshot_chunks_authoritative() {
        let chunks = Vec::new();
        let loaded = ChunkStorageKind::Snapshot
            .load_chunks(Path::new("unused"), 7, chunks)
            .expect("legacy snapshot storage must not require an external generation");
        assert!(loaded.is_empty());
    }

    #[test]
    fn legacy_snapshot_publication_does_not_require_external_storage() {
        let chunks = Vec::new();
        let snapshot_chunks = ChunkStorageKind::Snapshot
            .publish_chunks(Path::new("unused"), 7, &chunks)
            .expect("legacy publication must remain snapshot-owned");
        assert!(snapshot_chunks.is_empty());
    }

    #[test]
    fn legacy_snapshot_storage_never_occupies_or_removes_external_slots() {
        let missing = Path::new("this-path-is-never-read-for-snapshot-storage");
        assert!(!ChunkStorageKind::Snapshot
            .generation_slot_occupied(missing, 7)
            .expect("snapshot ownership must not inspect external storage"));
        ChunkStorageKind::Snapshot
            .remove_chunks(missing, 7)
            .expect("snapshot ownership must not remove external storage");
    }
}
