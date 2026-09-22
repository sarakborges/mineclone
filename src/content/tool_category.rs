use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::registry::DefinitionMap;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCategoryDefinition {
    pub id: String,
    pub name: LocalizedText,
}

#[derive(Resource, Default)]
pub struct ToolCategoryRegistry {
    definitions: DefinitionMap<ToolCategoryDefinition>,
}

impl ToolCategoryRegistry {
    pub fn insert(&mut self, mut definition: ToolCategoryDefinition) {
        definition.id = definition.id.trim().to_owned();
        assert!(!definition.id.is_empty(), "tool category id cannot be empty");
        definition
            .name
            .validate(&format!("tool category {} name", definition.id));
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&ToolCategoryDefinition> {
        self.definitions.get(id)
    }

}
