use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::structure::StructureRegistry;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureSetDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub structures: Vec<String>,
}

impl StructureSetDefinition {
    pub(crate) fn validate_references(&self, structures: &StructureRegistry) {
        for structure_id in &self.structures {
            assert!(
                structures.get(structure_id).is_some(),
                "structure set {} references missing structure: {structure_id}",
                self.id
            );
        }
    }
}

#[derive(Resource, Default)]
pub struct StructureSetRegistry {
    definitions: HashMap<String, StructureSetDefinition>,
}

impl StructureSetRegistry {
    pub fn insert(&mut self, definition: StructureSetDefinition) {
        assert!(
            !definition.id.trim().is_empty(),
            "structure set id cannot be empty"
        );
        assert!(
            !definition.name.trim().is_empty(),
            "structure set {} name cannot be empty",
            definition.id
        );

        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&StructureSetDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &StructureSetDefinition> {
        self.definitions.values()
    }
}
