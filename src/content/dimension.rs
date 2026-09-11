use bevy::prelude::*;
use serde::Deserialize;

use super::{dimension_hydrology::DimensionHydrology, registry::DefinitionMap};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionDefinition {
    pub id: String,
    pub name: String,
    pub biomes: Vec<String>,
    pub day_night_cycle: String,
    pub sky: String,
    pub sea_level: i32,
    #[serde(default)]
    pub hydrology: DimensionHydrology,
}

#[derive(Resource, Default)]
pub struct DimensionRegistry {
    definitions: DefinitionMap<DimensionDefinition>,
}

impl DimensionRegistry {
    pub fn insert(&mut self, definition: DimensionDefinition) {
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&DimensionDefinition> {
        self.definitions.get(id)
    }
}
