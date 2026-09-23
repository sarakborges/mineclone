use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::{Language, LocalizedText};

use super::{
    asset_path::is_safe_relative_asset_path,
    block::BlockRegistry,
    builtin_ids::ARTISANS_KIT_TOOL_ID,
    inventory_category::InventoryCategoryRegistry,
    item_id::intern_item_id,
    layer::LayerRegistry,
    registry::DefinitionMap,
    tool::ToolRegistry,
};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub category: String,
    pub icon: String,
}

impl ItemDefinition {
    pub(crate) fn validate_references(&self, categories: &InventoryCategoryRegistry) {
        assert!(
            categories.get(&self.category).is_some(),
            "item {} references missing inventory category {}",
            self.id,
            self.category
        );
    }
}

#[derive(Resource, Default)]
pub struct ItemRegistry {
    definitions: DefinitionMap<ItemDefinition>,
}

impl ItemRegistry {
    pub fn insert(&mut self, mut definition: ItemDefinition) {
        definition.id = definition.id.trim().to_owned();
        definition.category = definition.category.trim().to_owned();
        definition.icon = definition.icon.trim().to_owned();

        assert!(!definition.id.is_empty(), "item id cannot be empty");
        assert!(
            !definition.category.is_empty(),
            "item {} category cannot be empty",
            definition.id
        );
        assert!(
            !definition.icon.is_empty(),
            "item {} icon cannot be empty",
            definition.id
        );
        assert!(
            is_safe_relative_asset_path(&definition.icon),
            "item {} icon must be a safe relative asset path: {}",
            definition.id,
            definition.icon
        );
        definition
            .name
            .validate(&format!("item {} name", definition.id));
        intern_item_id(&definition.id);
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&ItemDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ItemDefinition> {
        self.definitions.values()
    }
}

pub(crate) fn display_name<'a>(
    item_id: &'a str,
    items: &'a ItemRegistry,
    blocks: &'a BlockRegistry,
    layers: &'a LayerRegistry,
    tools: &'a ToolRegistry,
    language: Language,
) -> &'a str {
    if let Some(item) = items.get(item_id) {
        return item.name.text(language);
    }
    if let Some(block) = blocks.get(item_id) {
        return block.name.text(language);
    }
    if let Some(layer) = layers.get(item_id) {
        return layer.name.text(language);
    }
    if let Some(tool) = tools.get(item_id) {
        return tool.name.text(language);
    }
    item_id
}
