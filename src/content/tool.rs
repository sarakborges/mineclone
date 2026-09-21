use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{
    asset_path::is_safe_relative_asset_path, registry::DefinitionMap, tool_id::intern_tool_id,
};

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
        definition.id = definition.id.trim().to_owned();
        definition.category = definition.category.trim().to_owned();
        definition.icon = definition.icon.trim().to_owned();
        definition.tint_icon = definition
            .tint_icon
            .map(|path| path.trim().to_owned())
            .filter(|path| !path.is_empty());

        assert!(!definition.id.is_empty(), "tool id cannot be empty");
        assert!(
            !definition.category.is_empty(),
            "tool {} category cannot be empty",
            definition.id
        );
        if !definition.icon.is_empty() {
            assert!(
                is_safe_relative_asset_path(&definition.icon),
                "tool {} icon must be a safe relative asset path: {}",
                definition.id,
                definition.icon
            );
        }
        if let Some(tint_icon) = definition.tint_icon.as_deref() {
            assert!(
                is_safe_relative_asset_path(tint_icon),
                "tool {} tintIcon must be a safe relative asset path: {tint_icon}",
                definition.id
            );
        }
        definition
            .name
            .validate(&format!("tool {} name", definition.id));
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


#[cfg(test)]
mod tests {
    use super::*;

    fn localized_name() -> LocalizedText {
        serde_json::from_str(
            r#"{"english":"Shears","portuguese_brazil":"Tesoura","spanish":"Tijeras"}"#,
        )
        .expect("test localization must parse")
    }

    #[test]
    fn tools_without_icons_are_valid_and_use_text_fallbacks() {
        let mut registry = ToolRegistry::default();
        registry.insert(ToolDefinition {
            id: "asteria:test_shears".to_owned(),
            name: localized_name(),
            category: "tools".to_owned(),
            icon: String::new(),
            tint_icon: None,
        });

        let tool = registry
            .get("asteria:test_shears")
            .expect("tool must be registered");
        assert!(tool.icon.is_empty());
    }
}
