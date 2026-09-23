use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{
    asset_path::is_safe_relative_asset_path,
    registry::DefinitionMap,
};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryCategoryDefinition {
    pub id: String,
    pub display_name: LocalizedText,
    pub order: u16,
    pub icon: String,
}

impl InventoryCategoryDefinition {
    pub fn validate(&self) {
        assert!(
            !self.id.trim().is_empty(),
            "inventory category id cannot be empty"
        );
        assert!(
            !self.icon.trim().is_empty(),
            "inventory category {} icon cannot be empty",
            self.id
        );
        assert!(
            is_safe_relative_asset_path(&self.icon),
            "inventory category {} icon must be a safe relative asset path: {}",
            self.id,
            self.icon
        );
        self.display_name
            .validate(&format!("inventory category {} display name", self.id));
    }
}

#[derive(Resource, Default)]
pub struct InventoryCategoryRegistry {
    definitions: DefinitionMap<InventoryCategoryDefinition>,
}

impl InventoryCategoryRegistry {
    pub fn insert(&mut self, mut definition: InventoryCategoryDefinition) {
        definition.id = definition.id.trim().to_owned();
        definition.icon = definition.icon.trim().to_owned();
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&InventoryCategoryDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &InventoryCategoryDefinition> {
        self.definitions.values()
    }
}
