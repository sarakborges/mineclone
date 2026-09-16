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
    #[serde(default)]
    pub tint_icon: Option<String>,
}

#[derive(Resource, Default)]
pub struct ToolRegistry {
    definitions: DefinitionMap<ToolDefinition>,
}

impl ToolRegistry {
    pub fn insert(&mut self, mut definition: ToolDefinition) {
        definition
            .name
            .validate(&format!("tool {} name", definition.id));
        definition.icon = definition.icon.trim().to_owned();
        definition.tint_icon = definition
            .tint_icon
            .map(|path| path.trim().to_owned())
            .filter(|path| !path.is_empty());
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
