use std::{cmp::Ordering, collections::BTreeMap, io};

use serde::{Deserialize, Serialize};

pub(crate) const MAX_STACK_SIZE: u32 = 64;

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
    quantity: u32,
    metadata: ItemMetadata,
}

impl ItemStack {
    pub(crate) fn new(id: &'static str) -> Self {
        Self {
            id,
            quantity: 1,
            metadata: ItemMetadata::default(),
        }
    }

    pub(crate) fn id(&self) -> &'static str {
        self.id
    }

    pub(crate) fn quantity(&self) -> u32 {
        self.quantity
    }

    pub(crate) fn decrement_quantity(&mut self) {
        assert!(
            self.quantity > 1,
            "cannot decrement an item stack below one"
        );
        self.quantity -= 1;
    }

    pub(crate) fn metadata(&self) -> &ItemMetadata {
        &self.metadata
    }

    pub(crate) fn with_quantity(mut self, quantity: u32) -> Self {
        assert!(
            (1..=MAX_STACK_SIZE).contains(&quantity),
            "item stack quantity must be between 1 and {MAX_STACK_SIZE}"
        );
        self.quantity = quantity;
        self
    }

    pub(crate) fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.set(key, value);
        self
    }

    pub(crate) fn can_stack_with(&self, other: &Self) -> bool {
        self.id == other.id && self.metadata == other.metadata
    }

    pub(crate) fn stacking_cmp(&self, other: &Self) -> Ordering {
        self.id
            .cmp(other.id)
            .then_with(|| self.metadata.cmp(&other.metadata))
    }

    pub(crate) fn merge_from(&mut self, mut incoming: Self) -> Option<Self> {
        assert!(
            self.can_stack_with(&incoming),
            "cannot merge item stacks with different identities"
        );
        let free = MAX_STACK_SIZE - self.quantity;
        let moved = free.min(incoming.quantity);
        self.quantity += moved;
        incoming.quantity -= moved;
        (incoming.quantity > 0).then_some(incoming)
    }

    pub(crate) fn saved(&self) -> SavedItemStack {
        if self.quantity == 1 && self.metadata.is_empty() {
            SavedItemStack::Id(self.id.to_owned())
        } else {
            SavedItemStack::Detailed {
                id: self.id.to_owned(),
                quantity: self.quantity,
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
        #[serde(
            default = "default_saved_quantity",
            skip_serializing_if = "saved_quantity_is_one"
        )]
        quantity: u32,
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

    pub(crate) fn quantity(&self) -> u32 {
        match self {
            Self::Id(_) => 1,
            Self::Detailed { quantity, .. } => *quantity,
        }
    }

    pub(crate) fn quantity_is_valid(&self) -> bool {
        (1..=MAX_STACK_SIZE).contains(&self.quantity())
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
        if !self.quantity_is_valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "item stack quantity must be between 1 and {MAX_STACK_SIZE}: {}",
                    self.quantity()
                ),
            ));
        }

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
        Ok(ItemStack {
            id,
            quantity: self.quantity(),
            metadata,
        })
    }
}

fn default_saved_quantity() -> u32 {
    1
}

fn saved_quantity_is_one(quantity: &u32) -> bool {
    *quantity == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_participates_in_stack_identity() {
        let plain = ItemStack::new("asteria:grass_block");
        let enchanted = ItemStack::new("asteria:grass_block")
            .with_metadata("biome_tint", "asteria:overworld/enchanted_forest");

        assert!(!plain.can_stack_with(&enchanted));
        assert!(enchanted.can_stack_with(&enchanted.clone()));
    }

    #[test]
    fn compatible_stacks_merge_up_to_limit_and_return_remainder() {
        let mut target = ItemStack::new("asteria:pebble").with_quantity(60);
        let incoming = ItemStack::new("asteria:pebble").with_quantity(10);

        let remainder = target.merge_from(incoming).expect("six items should remain");
        assert_eq!(target.quantity(), MAX_STACK_SIZE);
        assert_eq!(remainder.quantity(), 6);
    }

    #[test]
    fn metadata_free_single_save_keeps_legacy_string_shape() {
        let value = serde_json::to_value(ItemStack::new("asteria:grass_block").saved()).unwrap();
        assert_eq!(
            value,
            serde_json::Value::String("asteria:grass_block".to_owned())
        );
    }

    #[test]
    fn quantity_save_uses_detailed_shape() {
        let value =
            serde_json::to_value(ItemStack::new("asteria:pebble").with_quantity(4).saved())
                .unwrap();

        assert_eq!(value["id"], "asteria:pebble");
        assert_eq!(value["quantity"], 4);
        assert!(value.get("metadata").is_none());
    }

    #[test]
    fn old_detailed_save_without_quantity_defaults_to_one() {
        let saved: SavedItemStack = serde_json::from_value(serde_json::json!({
            "id": "asteria:grass_block",
            "metadata": {
                "biome_tint": "asteria:overworld/enchanted_forest"
            }
        }))
        .unwrap();

        assert_eq!(saved.quantity(), 1);
    }

    #[test]
    fn metadata_save_uses_detailed_shape() {
        let value = serde_json::to_value(
            ItemStack::new("asteria:grass_block")
                .with_metadata("biome_tint", "asteria:overworld/enchanted_forest")
                .saved(),
        )
        .unwrap();

        assert_eq!(value["id"], "asteria:grass_block");
        assert_eq!(
            value["metadata"]["biome_tint"],
            "asteria:overworld/enchanted_forest"
        );
    }
}
