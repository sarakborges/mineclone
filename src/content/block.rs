use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct BlockDefinition {
    pub id: String,
    pub name: String,
    pub texture: String,
    pub rotate_texture: bool,
}

#[derive(Resource, Default)]
pub struct BlockRegistry {
    definitions: HashMap<String, BlockDefinition>,
}

impl BlockRegistry {
    pub fn insert(&mut self, definition: BlockDefinition) {
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BlockDefinition> {
        self.definitions.get(id)
    }
}
