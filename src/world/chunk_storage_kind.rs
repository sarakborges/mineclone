use serde::{Deserialize, Serialize};

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
}
