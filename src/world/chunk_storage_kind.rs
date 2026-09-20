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

    /// Publishes the authoritative chunk artifact, runs the caller's remaining generation
    /// publication, and rolls the chunk artifact back if that publication fails.
    ///
    /// The save catalog owns snapshot/manifest commit ordering; this helper owns only the
    /// storage-specific rollback invariant so callers cannot forget to clean up a published
    /// external generation when a later commit-marker step fails.
    pub(crate) fn publish_chunks_then<T>(
        self,
        world_directory: &Path,
        generation: u64,
        chunks: &[DiskChunk],
        publish_generation: impl FnOnce(Vec<DiskChunk>) -> io::Result<T>,
    ) -> io::Result<T> {
        let snapshot_chunks = self.publish_chunks(world_directory, generation, chunks)?;
        match publish_generation(snapshot_chunks) {
            Ok(value) => Ok(value),
            Err(error) => {
                if let Err(cleanup_error) = self.remove_chunks(world_directory, generation) {
                    return Err(io::Error::other(format!(
                        "generation publication failed ({error}); chunk rollback also failed ({cleanup_error})"
                    )));
                }
                Err(error)
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

    /// Validates that the authoritative chunk artifact for a published generation can be
    /// read. Legacy snapshot ownership is validated when the snapshot itself is decoded;
    /// external ownership additionally requires its generation directory to be complete.
    pub(crate) fn validate_generation(
        self,
        world_directory: &Path,
        generation: u64,
    ) -> io::Result<()> {
        match self {
            Self::Snapshot => Ok(()),
            Self::GenerationDirectory => read_generation_chunks(world_directory, generation).map(|_| ()),
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

    fn temp_root(label: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "asteria-chunk-kind-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock must be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root must be created");
        root
    }

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

    #[test]
    fn legacy_snapshot_generation_validation_does_not_require_external_storage() {
        ChunkStorageKind::Snapshot
            .validate_generation(Path::new("this-path-is-never-read-for-snapshot-storage"), 7)
            .expect("legacy generation validation must stay snapshot-owned");
    }

    #[test]
    fn external_generation_validation_rejects_missing_storage() {
        let root = temp_root("missing");
        let error = ChunkStorageKind::GenerationDirectory
            .validate_generation(&root, 7)
            .expect_err("missing external generation must not validate");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        std::fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn external_generation_round_trip_keeps_snapshot_representation_empty() {
        let root = temp_root("round-trip");
        let chunk: DiskChunk = serde_json::from_str(r#"{"coord":[3,2,-1]}"#)
            .expect("minimal empty chunk fixture must decode");
        let storage = ChunkStorageKind::GenerationDirectory;

        let snapshot_chunks = storage
            .publish_chunks(&root, 11, std::slice::from_ref(&chunk))
            .expect("external chunks must publish");
        assert!(snapshot_chunks.is_empty());
        assert!(storage
            .generation_slot_occupied(&root, 11)
            .expect("published slot must be inspectable"));
        storage
            .validate_generation(&root, 11)
            .expect("published generation must validate");

        let loaded = storage
            .load_chunks(&root, 11, snapshot_chunks)
            .expect("external generation must restore");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].coord, [3, 2, -1]);

        storage
            .remove_chunks(&root, 11)
            .expect("external generation must be removable");
        assert!(!storage
            .generation_slot_occupied(&root, 11)
            .expect("removed slot must be inspectable"));
        std::fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn external_generation_rejects_snapshot_chunks_as_duplicate_owner() {
        let root = temp_root("duplicate-owner");
        let chunk: DiskChunk = serde_json::from_str(r#"{"coord":[0,0,0]}"#)
            .expect("minimal empty chunk fixture must decode");
        ChunkStorageKind::GenerationDirectory
            .publish_chunks(&root, 5, std::slice::from_ref(&chunk))
            .expect("external chunks must publish");

        let error = ChunkStorageKind::GenerationDirectory
            .load_chunks(&root, 5, vec![chunk])
            .expect_err("dual ownership must be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);

        ChunkStorageKind::GenerationDirectory
            .remove_chunks(&root, 5)
            .expect("external generation must be removable");
        std::fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn failed_generation_publication_rolls_back_external_chunks() {
        let root = temp_root("rollback");
        let chunk: DiskChunk = serde_json::from_str(r#"{"coord":[1,0,2]}"#)
            .expect("minimal empty chunk fixture must decode");
        let storage = ChunkStorageKind::GenerationDirectory;

        let error = storage
            .publish_chunks_then(&root, 13, std::slice::from_ref(&chunk), |snapshot_chunks| {
                assert!(snapshot_chunks.is_empty());
                Err::<(), _>(io::Error::other("commit marker failed"))
            })
            .expect_err("failed generation publication must propagate");
        assert_eq!(error.to_string(), "commit marker failed");
        assert!(!storage
            .generation_slot_occupied(&root, 13)
            .expect("rollback must release the generation slot"));
        std::fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn successful_generation_publication_keeps_external_chunks() {
        let root = temp_root("commit");
        let chunk: DiskChunk = serde_json::from_str(r#"{"coord":[2,0,4]}"#)
            .expect("minimal empty chunk fixture must decode");
        let storage = ChunkStorageKind::GenerationDirectory;

        let value = storage
            .publish_chunks_then(&root, 17, std::slice::from_ref(&chunk), |snapshot_chunks| {
                assert!(snapshot_chunks.is_empty());
                Ok(42)
            })
            .expect("successful generation publication must commit");
        assert_eq!(value, 42);
        assert!(storage
            .generation_slot_occupied(&root, 17)
            .expect("committed generation must keep its slot"));

        storage
            .remove_chunks(&root, 17)
            .expect("test generation must be removable");
        std::fs::remove_dir_all(root).expect("temp root must be removed");
    }
}
