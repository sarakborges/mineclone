use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct DimensionDefinition {
    pub id: String,
    pub name: String,
    pub biomes: Vec<String>,
    pub day_night_cycle: String,
    pub sky: String,
    pub sea_level: i32,
}

#[derive(Resource, Default)]
pub struct DimensionRegistry {
    definitions: HashMap<String, DimensionDefinition>,
}

impl DimensionRegistry {
    pub fn insert(&mut self, definition: DimensionDefinition) {
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&DimensionDefinition> {
        self.definitions.get(id)
    }
}
