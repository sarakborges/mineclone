use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{
    asset_path::is_safe_relative_asset_path, registry::DefinitionMap, tool_id::intern_tool_id,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolMiningDefinition {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_mining_speed")]
    pub speed: f32,
}

impl Default for ToolMiningDefinition {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            speed: default_mining_speed(),
        }
    }
}

impl ToolMiningDefinition {
    fn validate(&self, tool_id: &str) {
        assert!(
            self.speed.is_finite() && self.speed > 0.0,
            "tool {tool_id} mining speed must be finite and greater than zero"
        );
        for (index, tag) in self.tags.iter().enumerate() {
            assert!(
                !tag.trim().is_empty(),
                "tool {tool_id} mining tags cannot contain empty values"
            );
            assert!(
                !self.tags[..index].contains(tag),
                "tool {tool_id} mining tags cannot contain duplicates"
            );
        }
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|candidate| candidate == tag)
    }
}

fn default_mining_speed() -> f32 {
    1.0
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub category: String,
    pub icon: String,
    #[serde(default)]
    pub tint_icon: Option<String>,
    #[serde(default)]
    pub mining: ToolMiningDefinition,
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
        for tag in &mut definition.mining.tags {
            *tag = tag.trim().to_owned();
        }

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
        definition.mining.validate(&definition.id);
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
            mining: ToolMiningDefinition::default(),
        });

        let tool = registry
            .get("asteria:test_shears")
            .expect("tool must be registered");
        assert!(tool.icon.is_empty());
    }
}
