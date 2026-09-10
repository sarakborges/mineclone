use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureDefinition {
    pub id: String,
    pub name: String,
}

#[derive(Resource, Default)]
pub struct StructureRegistry {
    definitions: HashMap<String, StructureDefinition>,
}

impl StructureRegistry {
    pub fn insert(&mut self, definition: StructureDefinition) {
        assert!(
            !definition.id.trim().is_empty(),
            "structure id cannot be empty"
        );
        assert!(
            !definition.name.trim().is_empty(),
            "structure {} name cannot be empty",
            definition.id
        );

        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&StructureDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &StructureDefinition> {
        self.definitions.values()
    }
}
