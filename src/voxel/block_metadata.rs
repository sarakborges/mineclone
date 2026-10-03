use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Arbitrary persistent data attached to a block.
///
/// Metadata intentionally lives outside [`VoxelCell`](super::cell::VoxelCell):
/// most voxels never need it, and arbitrary values must not inflate the hot
/// block palette/state representation.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct BlockMetadata {
    values: Map<String, Value>,
}

impl BlockMetadata {
    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    pub(crate) fn insert_value(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        let key = key.into();
        assert!(!key.trim().is_empty(), "block metadata key cannot be empty");
        self.values.insert(key, value)
    }

    pub(crate) fn insert<T: Serialize>(
        &mut self,
        key: impl Into<String>,
        value: T,
    ) -> Result<Option<Value>, serde_json::Error> {
        Ok(self.insert_value(key, serde_json::to_value(value)?))
    }

    pub(crate) fn remove(&mut self, key: &str) -> Option<Value> {
        self.values.remove(key)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.values.iter().map(|(key, value)| (key.as_str(), value))
    }
}

/// Sparse metadata for one chunk, keyed by the local linear voxel index.
/// Empty metadata is never retained.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct BlockMetadataStore {
    entries: BTreeMap<u16, BlockMetadata>,
}

impl BlockMetadataStore {
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn get(&self, voxel_index: u16) -> Option<&BlockMetadata> {
        self.entries.get(&voxel_index)
    }

    pub(crate) fn get_mut(&mut self, voxel_index: u16) -> Option<&mut BlockMetadata> {
        self.entries.get_mut(&voxel_index)
    }

    pub(crate) fn set(&mut self, voxel_index: u16, metadata: BlockMetadata) {
        if metadata.is_empty() {
            self.entries.remove(&voxel_index);
        } else {
            self.entries.insert(voxel_index, metadata);
        }
    }

    pub(crate) fn remove(&mut self, voxel_index: u16) -> Option<BlockMetadata> {
        self.entries.remove(&voxel_index)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (u16, &BlockMetadata)> {
        self.entries
            .iter()
            .map(|(&voxel_index, metadata)| (voxel_index, metadata))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{BlockMetadata, BlockMetadataStore};

    #[test]
    fn metadata_accepts_structured_values() {
        let mut metadata = BlockMetadata::default();
        metadata.insert_value("inventory", json!({"slots": [1, 2, 3]}));

        assert_eq!(
            metadata.get("inventory"),
            Some(&json!({"slots": [1, 2, 3]}))
        );
    }

    #[test]
    fn sparse_store_drops_empty_metadata() {
        let mut store = BlockMetadataStore::default();
        store.set(7, BlockMetadata::default());

        assert!(store.is_empty());

        let mut metadata = BlockMetadata::default();
        metadata.insert_value("custom_name", json!("Crate"));
        store.set(7, metadata);

        assert_eq!(store.len(), 1);
        assert_eq!(
            store.get(7).and_then(|data| data.get("custom_name")),
            Some(&json!("Crate"))
        );
    }
}
