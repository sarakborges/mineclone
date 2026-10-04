use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use crate::content::registry::DefinitionMap;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub day_night_cycle: String,
    pub sky: String,
    pub sea_level: i32,
    pub gravity_strength: f32,
    #[serde(default = "default_max_entities")]
    pub max_entities: usize,
}

fn default_max_entities() -> usize {
    128
}

#[derive(Resource, Default)]
pub struct DimensionRegistry {
    definitions: DefinitionMap<DimensionDefinition>,
}

impl DimensionRegistry {
    pub fn insert(&mut self, definition: DimensionDefinition) {
        assert!(!definition.id.trim().is_empty(), "dimension id cannot be empty");
        definition
            .name
            .validate(&format!("dimension {} name", definition.id));
        assert!(
            !definition.day_night_cycle.trim().is_empty(),
            "dimension {} dayNightCycle cannot be empty",
            definition.id
        );
        assert!(
            !definition.sky.trim().is_empty(),
            "dimension {} sky cannot be empty",
            definition.id
        );
        assert!(
            definition.gravity_strength.is_finite() && definition.gravity_strength >= 0.0,
            "dimension {} gravityStrength must be finite and non-negative",
            definition.id
        );
        assert!(
            definition.max_entities > 0,
            "dimension {} maxEntities must be positive",
            definition.id
        );
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&DimensionDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DimensionDefinition> {
        self.definitions.values()
    }
}
