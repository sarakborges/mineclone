use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{
    block::{BlockDefinition, BlockRegistry, BlockTint},
    color::Hsi,
    registry::DefinitionMap,
};

const DEFAULT_GRASS_ICON_TINT: Hsi = Hsi::new(112.1111, 0.56363636, 0.36666667);
const DEFAULT_LEAF_ICON_TINT: Hsi = Hsi::new(112.1111, 0.56363636, 0.36666667);
const DEFAULT_FOLIAGE_ICON_TINT: Hsi = Hsi::new(112.1111, 0.56363636, 0.36666667);

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryCategoryBlockIcon {
    pub block: String,
    #[serde(default)]
    pub tint: Option<Hsi>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryCategoryDefinition {
    pub id: String,
    pub display_name: LocalizedText,
    pub order: u16,
    #[serde(default)]
    pub block_icon: Option<InventoryCategoryBlockIcon>,
}

impl InventoryCategoryDefinition {
    pub fn validate_references(&self, blocks: &BlockRegistry) {
        self.display_name
            .validate(&format!("inventory category {} display name", self.id));
        if let Some(block_icon) = &self.block_icon {
            assert!(
                blocks.get(&block_icon.block).is_some(),
                "inventory category {} references missing block icon {}",
                self.id,
                block_icon.block
            );
        }
    }

    pub fn icon_tint(&self, block: &BlockDefinition) -> Color {
        if let Some(tint) = self.block_icon.as_ref().and_then(|icon| icon.tint) {
            return tint.to_color();
        }

        match block.tint {
            BlockTint::None => Color::WHITE,
            BlockTint::Grass => DEFAULT_GRASS_ICON_TINT.to_color(),
            BlockTint::Leaf => DEFAULT_LEAF_ICON_TINT.to_color(),
            BlockTint::Foliage => DEFAULT_FOLIAGE_ICON_TINT.to_color(),
        }
    }
}

#[derive(Resource, Default)]
pub struct InventoryCategoryRegistry {
    definitions: DefinitionMap<InventoryCategoryDefinition>,
}

impl InventoryCategoryRegistry {
    pub fn insert(&mut self, definition: InventoryCategoryDefinition) {
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&InventoryCategoryDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &InventoryCategoryDefinition> {
        self.definitions.values()
    }
}
