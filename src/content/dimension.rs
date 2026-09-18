use std::collections::HashSet;

use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{
    biome::{BiomeKind, BiomeRegistry},
    dimension_hydrology::DimensionHydrology,
    registry::DefinitionMap,
};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionBiomeSizeAxis {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionBiomeSize {
    pub x: DimensionBiomeSizeAxis,
    pub z: DimensionBiomeSizeAxis,
    #[serde(default)]
    pub y: Option<DimensionBiomeSizeAxis>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionBiome {
    pub id: String,
    #[serde(default = "default_biome_weight")]
    pub weight: f32,
    #[serde(default)]
    pub size: Option<DimensionBiomeSize>,
    #[serde(default)]
    pub avoid_near: Vec<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub biomes: Vec<DimensionBiome>,
    pub day_night_cycle: String,
    pub sky: String,
    pub sea_level: i32,
    #[serde(default = "default_max_entities")]
    pub max_entities: usize,
    #[serde(default)]
    pub hydrology: DimensionHydrology,
}

fn default_max_entities() -> usize { 128 }

impl DimensionDefinition {
    pub fn validate_biomes(&self, biomes: &BiomeRegistry) {
        assert!(self.max_entities > 0, "dimension {} maxEntities must be positive", self.id);
        assert!(
            !self.biomes.is_empty(),
            "dimension {} must define at least one biome",
            self.id
        );

        let mut ids = HashSet::new();
        let mut has_active_surface = false;

        for entry in &self.biomes {
            assert!(
                ids.insert(entry.id.as_str()),
                "dimension {} defines biome more than once: {}",
                self.id,
                entry.id
            );
            assert!(
                entry.weight.is_finite() && entry.weight >= 0.0,
                "dimension {} biome {} weight must be finite and non-negative",
                self.id,
                entry.id
            );

            let biome = biomes.get(&entry.id).unwrap_or_else(|| {
                panic!(
                    "dimension {} references missing biome: {}",
                    self.id, entry.id
                )
            });

            match biome.kind {
                BiomeKind::Surface => {
                    let size = entry.size.unwrap_or_else(|| {
                        panic!(
                            "dimension {} surface biome {} must define size",
                            self.id, entry.id
                        )
                    });
                    validate_size_axis(&self.id, &entry.id, "x", size.x);
                    validate_size_axis(&self.id, &entry.id, "z", size.z);
                    if let Some(vertical_size) = size.y {
                        validate_size_axis(&self.id, &entry.id, "y", vertical_size);
                    }

                    if entry.weight > 0.0 {
                        has_active_surface = true;
                    }
                }
                BiomeKind::Volume => {
                    let size = entry.size.unwrap_or_else(|| {
                        panic!(
                            "dimension {} volume biome {} must define size",
                            self.id, entry.id
                        )
                    });
                    validate_size_axis(&self.id, &entry.id, "x", size.x);
                    validate_size_axis(&self.id, &entry.id, "z", size.z);
                    let vertical_size = size.y.unwrap_or_else(|| {
                        panic!(
                            "dimension {} volume biome {} must define size.y",
                            self.id, entry.id
                        )
                    });
                    validate_size_axis(&self.id, &entry.id, "y", vertical_size);
                }
                BiomeKind::Hydrology => {
                    assert!(
                        entry.size.is_none(),
                        "dimension {} hydrology biome {} cannot define size",
                        self.id,
                        entry.id
                    );
                }
            }
        }

        assert!(
            has_active_surface,
            "dimension {} must define at least one surface biome with positive weight",
            self.id
        );

        for entry in &self.biomes {
            let mut avoided = HashSet::new();
            for avoided_id in &entry.avoid_near {
                assert!(
                    avoided.insert(avoided_id.as_str()),
                    "dimension {} biome {} avoidNear cannot contain duplicates: {}",
                    self.id,
                    entry.id,
                    avoided_id
                );
                assert!(
                    avoided_id != &entry.id,
                    "dimension {} biome {} cannot avoid itself",
                    self.id,
                    entry.id
                );
                assert!(
                    self.biomes.iter().any(|candidate| candidate.id == *avoided_id),
                    "dimension {} biome {} avoidNear references missing biome: {}",
                    self.id,
                    entry.id,
                    avoided_id
                );
            }
        }

        for (field, biome_id) in [
            ("hydrology.oceanBiome", self.hydrology.ocean_biome.as_deref()),
            ("hydrology.coastBiome", self.hydrology.coast_biome.as_deref()),
        ] {
            let Some(biome_id) = biome_id else {
                continue;
            };
            assert!(
                self.biomes.iter().any(|entry| entry.id == biome_id),
                "dimension {} {field} must also be listed in biomes: {biome_id}",
                self.id
            );
        }
    }

    pub fn biome_weight(&self, biome_id: &str) -> f32 {
        self.biomes
            .iter()
            .find(|entry| entry.id == biome_id)
            .map(|entry| entry.weight)
            .unwrap_or_else(|| {
                panic!(
                    "dimension {} does not define biome weight for {biome_id}",
                    self.id
                )
            })
    }
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

    pub fn iter(&self) -> impl Iterator<Item = &DimensionDefinition> {
        self.definitions.values()
    }
}

fn validate_size_axis(
    dimension_id: &str,
    biome_id: &str,
    axis: &str,
    size: DimensionBiomeSizeAxis,
) {
    assert!(
        size.min > 0.0,
        "dimension {dimension_id} biome {biome_id} size.{axis}.min must be positive"
    );
    assert!(
        size.max >= size.min,
        "dimension {dimension_id} biome {biome_id} size.{axis}.max must be greater than or equal to min"
    );
}

fn default_biome_weight() -> f32 {
    1.0
}
