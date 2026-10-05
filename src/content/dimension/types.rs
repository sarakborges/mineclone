use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use crate::content::registry::DefinitionMap;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedOceanDefinition {
    pub biome: String,
    pub fluid: String,
}

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
    #[serde(default)]
    pub generated_ocean: Option<GeneratedOceanDefinition>,
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
        if let Some(ocean) = &definition.generated_ocean {
            assert_namespaced_id(
                &definition.id,
                "generatedOcean.biome",
                &ocean.biome,
            );
            assert_namespaced_id(
                &definition.id,
                "generatedOcean.fluid",
                &ocean.fluid,
            );
        }
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&DimensionDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DimensionDefinition> {
        self.definitions.values()
    }
}

fn assert_namespaced_id(dimension_id: &str, field: &str, value: &str) {
    let trimmed = value.trim();
    let valid = trimmed.split_once(':').is_some_and(|(namespace, local)| {
        !namespace.is_empty() && !local.is_empty() && !local.contains(':')
    });
    assert!(
        valid && trimmed == value,
        "dimension {dimension_id} {field} must be a trimmed namespaced id"
    );
}
