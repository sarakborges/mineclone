use std::{collections::BTreeMap, io};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ItemMetadata {
    values: BTreeMap<String, String>,
}

impl ItemMetadata {
    pub(crate) fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub(crate) fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        assert!(!key.trim().is_empty(), "item metadata key cannot be empty");
        assert!(!value.trim().is_empty(), "item metadata value cannot be empty");
        self.values.insert(key, value);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    fn from_saved(values: BTreeMap<String, String>) -> io::Result<Self> {
        if values
            .iter()
            .any(|(key, value)| key.trim().is_empty() || value.trim().is_empty())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "item metadata keys and values cannot be empty",
            ));
        }
        Ok(Self { values })
    }

    fn to_saved(&self) -> BTreeMap<String, String> {
        self.values.clone()
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ItemStack {
    id: &'static str,
    metadata: ItemMetadata,
}

impl ItemStack {
    pub(crate) fn new(id: &'static str) -> Self {
        Self {
            id,
            metadata: ItemMetadata::default(),
        }
    }

    pub(crate) fn id(&self) -> &'static str {
        self.id
    }

    pub(crate) fn metadata(&self) -> &ItemMetadata {
        &self.metadata
    }

    pub(crate) fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.set(key, value);
        self
    }

    pub(crate) fn saved(&self) -> SavedItemStack {
        if self.metadata.is_empty() {
            SavedItemStack::Id(self.id.to_owned())
        } else {
            SavedItemStack::Detailed {
                id: self.id.to_owned(),
                metadata: self.metadata.to_saved(),
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum SavedItemStack {
    Id(String),
    Detailed {
        id: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        metadata: BTreeMap<String, String>,
    },
}

impl SavedItemStack {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Id(id) | Self::Detailed { id, .. } => id,
        }
    }

    pub(crate) fn metadata_value(&self, key: &str) -> Option<&str> {
        match self {
            Self::Id(_) => None,
            Self::Detailed { metadata, .. } => metadata.get(key).map(String::as_str),
        }
    }

    pub(crate) fn metadata_is_valid(&self) -> bool {
        match self {
            Self::Id(_) => true,
            Self::Detailed { metadata, .. } => metadata
                .iter()
                .all(|(key, value)| !key.trim().is_empty() && !value.trim().is_empty()),
        }
    }

    pub(crate) fn restore(
        &self,
        resolve_id: impl FnOnce(&str) -> Option<&'static str>,
    ) -> io::Result<ItemStack> {
        let id = resolve_id(self.id()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown inventory item ID: {}", self.id()),
            )
        })?;
        let metadata = match self {
            Self::Id(_) => ItemMetadata::default(),
            Self::Detailed { metadata, .. } => ItemMetadata::from_saved(metadata.clone())?,
        };
        Ok(ItemStack { id, metadata })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_participates_in_stack_identity() {
        let plain = ItemStack::new("asteria:grass");
        let enchanted = ItemStack::new("asteria:grass")
            .with_metadata("biome_tint", "asteria:overworld/enchanted_forest");

        assert_ne!(plain, enchanted);
        assert_eq!(enchanted, enchanted.clone());
    }

    #[test]
    fn metadata_free_save_keeps_legacy_string_shape() {
        let value = serde_json::to_value(ItemStack::new("asteria:grass").saved()).unwrap();
        assert_eq!(value, serde_json::Value::String("asteria:grass".to_owned()));
    }

    #[test]
    fn metadata_save_uses_detailed_shape() {
        let value = serde_json::to_value(
            ItemStack::new("asteria:grass")
                .with_metadata("biome_tint", "asteria:overworld/enchanted_forest")
                .saved(),
        )
        .unwrap();

        assert_eq!(value["id"], "asteria:grass");
        assert_eq!(
            value["metadata"]["biome_tint"],
            "asteria:overworld/enchanted_forest"
        );
    }
}
