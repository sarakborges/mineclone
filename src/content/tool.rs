use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{registry::DefinitionMap, tool_id::intern_tool_id};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub category: String,
    pub icon: String,
}

#[derive(Resource, Default)]
pub struct ToolRegistry {
    definitions: DefinitionMap<ToolDefinition>,
}

impl ToolRegistry {
    pub fn insert(&mut self, definition: ToolDefinition) {
        definition
            .name
            .validate(&format!("tool {} name", definition.id));
        assert!(
            !definition.icon.trim().is_empty(),
            "tool {} icon cannot be empty",
            definition.id
        );
        intern_tool_id(&definition.id);
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&ToolDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ToolDefinition> {
        self.definitions.values()
    }
}
