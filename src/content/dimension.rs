use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{dimension_hydrology::DimensionHydrology, registry::DefinitionMap};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionDefinition {
    pub id: String,
    pub name: LocalizedText,
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
        definition
            .name
            .validate(&format!("dimension {} name", definition.id));
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&DimensionDefinition> {
        self.definitions.get(id)
    }
}
