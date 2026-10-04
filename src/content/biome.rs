use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::registry::DefinitionMap;

/// Temporary identity-only biome content boundary.
///
/// Phase 1 deliberately deleted the legacy biome-generation schema. Phase 3
/// will define the new authored biome-layout contract from scratch. Until then
/// this registry exists only so unrelated content can continue referring to
/// biome ids without inheriting any generation behavior.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeDefinition {
    pub id: String,
    pub name: LocalizedText,
}

#[derive(Clone, Resource, Default)]
pub struct BiomeRegistry {
    definitions: DefinitionMap<BiomeDefinition>,
}

impl BiomeRegistry {
    pub fn insert(&mut self, definition: BiomeDefinition) {
        assert!(!definition.id.trim().is_empty(), "biome id cannot be empty");
        definition
            .name
            .validate(&format!("biome {} name", definition.id));
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BiomeDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &BiomeDefinition> {
        self.definitions.values()
    }
}
