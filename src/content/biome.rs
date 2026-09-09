use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::{
    biome_density::BiomeDensityModifier,
    biome_sky_layer::BiomeSkyLayerVisuals,
    biome_terrain::BiomeTerrain,
    color::Rgb,
    day_night_phase::DayNightPhases,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BiomeKind {
    #[default]
    Surface,
    Volume,
}

#[derive(Clone, Copy, Deserialize)]
pub struct BiomeSizeAxis {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Deserialize)]
pub struct BiomeSize {
    pub x: BiomeSizeAxis,
    pub z: BiomeSizeAxis,
    #[serde(default)]
    pub y: Option<BiomeSizeAxis>,
}

#[derive(Clone, Copy, Deserialize)]
pub struct BiomeClimateRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Default, Deserialize)]
pub struct BiomeClimate {
    #[serde(default)]
    pub temperature: Option<BiomeClimateRange>,
    #[serde(default)]
    pub humidity: Option<BiomeClimateRange>,
    #[serde(default)]
    pub continentalness: Option<BiomeClimateRange>,
    #[serde(default)]
    pub erosion: Option<BiomeClimateRange>,
}

#[derive(Clone, Copy, Deserialize)]
pub struct BiomeVerticalRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Deserialize)]
pub struct BiomeUnderwaterTint {
    pub color: Rgb,
    pub opacity: f32,
}

#[derive(Clone, Deserialize)]
pub struct BiomeVisuals {
    pub sky_color: DayNightPhases<Rgb>,
    pub fog_color: DayNightPhases<Rgb>,
    pub grass_color: Rgb,
    pub underwater_tint: BiomeUnderwaterTint,
    #[serde(default)]
    pub stars: BiomeSkyLayerVisuals,
    #[serde(default)]
    pub clouds: BiomeSkyLayerVisuals,
    pub terrain_roughness: f32,
    pub terrain_metallic: f32,
}

#[derive(Clone, Deserialize)]
pub struct BiomeDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: BiomeKind,
    pub size: BiomeSize,
    #[serde(default)]
    pub climate: BiomeClimate,
    #[serde(default)]
    pub vertical_range: Option<BiomeVerticalRange>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub terrain: Option<BiomeTerrain>,
    #[serde(default)]
    pub density_modifier: Option<BiomeDensityModifier>,
    #[serde(default)]
    pub solid_block: Option<String>,
    pub visuals: BiomeVisuals,
}

#[derive(Resource, Default)]
pub struct BiomeRegistry {
    definitions: HashMap<String, BiomeDefinition>,
}

impl BiomeRegistry {
    pub fn insert(&mut self, definition: BiomeDefinition) {
        validate_size_axis(&definition.id, "x", definition.size.x);
        validate_size_axis(&definition.id, "z", definition.size.z);

        if let Some(vertical_size) = definition.size.y {
            validate_size_axis(&definition.id, "y", vertical_size);
        }

        match definition.kind {
            BiomeKind::Surface => {
                assert!(
                    definition.terrain.is_some(),
                    "surface biome {} must define terrain",
                    definition.id
                );
                assert!(
                    definition.density_modifier.is_none(),
                    "surface biome {} cannot define a volume density modifier",
                    definition.id
                );
                assert_eq!(
                    definition.priority, 0,
                    "surface biome {} cannot define volume overlap priority",
                    definition.id
                );
            }
            BiomeKind::Volume => {
                assert!(
                    definition.size.y.is_some(),
                    "volume biome {} must define size.y",
                    definition.id
                );
            }
        }

        if let Some(range) = definition.vertical_range {
            assert!(
                range.min >= 0.0,
                "biome {} vertical_range.min cannot be negative",
                definition.id
            );
            assert!(
                range.max >= range.min,
                "biome {} vertical_range.max must be greater than or equal to min",
                definition.id
            );
        }

        validate_climate(&definition.id, definition.climate);

        assert!(
            (0.0..=1.0).contains(&definition.visuals.stars.density),
            "biome {} stars density must be between 0 and 1",
            definition.id
        );
        assert!(
            (0.0..=1.0).contains(&definition.visuals.clouds.density),
            "biome {} clouds density must be between 0 and 1",
            definition.id
        );
        assert!(
            (0.0..=1.0).contains(&definition.visuals.underwater_tint.opacity),
            "biome {} underwater tint opacity must be between 0 and 1",
            definition.id
        );

        if let Some(terrain) = &definition.terrain {
            terrain.validate(&definition.id);
        }

        if let Some(modifier) = &definition.density_modifier {
            modifier.validate(&definition.id);
        }

        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BiomeDefinition> {
        self.definitions.get(id)
    }

    pub fn has_volume_density_modifiers(&self) -> bool {
        self.definitions.values().any(|definition| {
            definition.kind == BiomeKind::Volume && definition.density_modifier.is_some()
        })
    }
}

fn validate_size_axis(biome_id: &str, axis: &str, size: BiomeSizeAxis) {
    assert!(
        size.min > 0.0,
        "biome {biome_id} size.{axis}.min must be positive"
    );
    assert!(
        size.max >= size.min,
        "biome {biome_id} size.{axis}.max must be greater than or equal to min"
    );
}

fn validate_climate(biome_id: &str, climate: BiomeClimate) {
    validate_climate_range(biome_id, "temperature", climate.temperature);
    validate_climate_range(biome_id, "humidity", climate.humidity);
    validate_climate_range(biome_id, "continentalness", climate.continentalness);
    validate_climate_range(biome_id, "erosion", climate.erosion);
}

fn validate_climate_range(
    biome_id: &str,
    field: &str,
    range: Option<BiomeClimateRange>,
) {
    let Some(range) = range else {
        return;
    };

    assert!(
        (0.0..=1.0).contains(&range.min),
        "biome {biome_id} climate.{field}.min must be between 0 and 1"
    );
    assert!(
        (0.0..=1.0).contains(&range.max),
        "biome {biome_id} climate.{field}.max must be between 0 and 1"
    );
    assert!(
        range.max >= range.min,
        "biome {biome_id} climate.{field}.max must be greater than or equal to min"
    );
}
