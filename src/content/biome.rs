use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use self::validation::validate_biome_definition;
use super::{
    biome_density::BiomeDensityModifier, biome_distribution::BiomeDistribution,
    biome_hydrology::BiomeHydrology, biome_material::BiomeMaterialLayer,
    biome_sky_layer::BiomeSkyLayerVisuals,
    biome_structure::{BiomeStructure, StructurePlacementRules},
    biome_surface_carver::BiomeSurfaceCarver, biome_terrain::BiomeTerrain,
    biome_terrain_modifier::BiomeTerrainModifier, color::Hsi, day_night_phase::DayNightPhases,
    registry::DefinitionMap,
};

mod validation;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BiomeKind {
    #[default]
    Surface,
    TerrainOverlay,
    Volume,
    Hydrology,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeClimateRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct BiomeVerticalRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeUnderwaterTint {
    pub color: Hsi,
    pub opacity: f32,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeVisuals {
    pub sky_color: DayNightPhases<Hsi>,
    pub fog_color: DayNightPhases<Hsi>,
    #[serde(default = "default_vegetation_color")]
    pub grass_color: Hsi,
    #[serde(default = "default_vegetation_color")]
    pub leaf_color: Hsi,
    #[serde(default = "default_vegetation_color")]
    pub foliage_color: Hsi,
    #[serde(default)]
    pub water_color: Option<Hsi>,
    pub underwater_tint: BiomeUnderwaterTint,
    #[serde(default)]
    pub stars: BiomeSkyLayerVisuals,
    #[serde(default)]
    pub clouds: BiomeSkyLayerVisuals,
    pub terrain_roughness: f32,
    pub terrain_metallic: f32,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatureSpawnRule {
    pub creature: String,
    #[serde(default = "default_spawn_weight")]
    pub weight: f32,
    #[serde(default)]
    pub light_min: u8,
    #[serde(default = "default_spawn_light_max")]
    pub light_max: u8,
    #[serde(default = "default_spawn_spacing")]
    pub spacing: f32,
}

fn default_spawn_weight() -> f32 { 1.0 }
fn default_spawn_light_max() -> u8 { 15 }
fn default_spawn_spacing() -> f32 { 16.0 }

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeDefinition {
    pub id: String,
    pub name: LocalizedText,
    #[serde(default)]
    pub kind: BiomeKind,
    #[serde(default)]
    pub parent_biome: Option<String>,
    #[serde(default = "default_biome_distributions")]
    pub distributions: Vec<BiomeDistribution>,
    #[serde(default)]
    pub climate: BiomeClimate,
    #[serde(default)]
    pub vertical_range: Option<BiomeVerticalRange>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub terrain: Option<BiomeTerrain>,
    #[serde(default)]
    pub terrain_modifiers: Vec<BiomeTerrainModifier>,
    #[serde(default)]
    pub allow_surface_carvers: bool,
    #[serde(default)]
    pub surface_carvers: Vec<BiomeSurfaceCarver>,
    #[serde(default)]
    pub surface_layers: Vec<BiomeMaterialLayer>,
    #[serde(default)]
    pub density_modifier: Option<BiomeDensityModifier>,
    #[serde(default)]
    pub solid_block: Option<String>,
    #[serde(default)]
    pub hydrology: BiomeHydrology,
    #[serde(default)]
    pub structures: Vec<BiomeStructure>,
    #[serde(default)]
    pub creature_spawns: Vec<CreatureSpawnRule>,
    #[serde(default)]
    pub visuals: Option<BiomeVisuals>,
}

impl BiomeDefinition {
    pub fn visuals(&self) -> &BiomeVisuals {
        self.visuals
            .as_ref()
            .unwrap_or_else(|| panic!("biome {} does not define visuals", self.id))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BiomeStructurePlacement {
    pub(crate) biome_id: String,
    pub(crate) structure_id: String,
    pub(crate) placement: StructurePlacementRules,
}

#[derive(Clone, Resource, Default)]
pub struct BiomeRegistry {
    definitions: DefinitionMap<BiomeDefinition>,
    has_volume_density_modifiers: bool,
    has_volume_solid_density_modifiers: bool,
    structure_placements: Vec<BiomeStructurePlacement>,
}

impl BiomeRegistry {
    pub fn insert(&mut self, definition: BiomeDefinition) {
        definition
            .name
            .validate(&format!("biome {} name", definition.id));
        validate_biome_definition(&definition);
        self.definitions.insert(definition.id.clone(), definition);
        self.rebuild_runtime_metadata();
    }

    pub fn get(&self, id: &str) -> Option<&BiomeDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &BiomeDefinition> {
        self.definitions.values()
    }

    pub fn has_volume_density_modifiers(&self) -> bool {
        self.has_volume_density_modifiers
    }

    pub fn has_volume_solid_density_modifiers(&self) -> bool {
        self.has_volume_solid_density_modifiers
    }

    pub(crate) fn structure_placements(&self) -> &[BiomeStructurePlacement] {
        &self.structure_placements
    }

    fn rebuild_runtime_metadata(&mut self) {
        self.has_volume_density_modifiers = self.definitions.values().any(|definition| {
            definition.kind == BiomeKind::Volume && definition.density_modifier.is_some()
        });
        self.has_volume_solid_density_modifiers = self.definitions.values().any(|definition| {
            definition.kind == BiomeKind::Volume
                && matches!(
                    definition.density_modifier,
                    Some(BiomeDensityModifier::Solid { .. })
                )
        });

        self.structure_placements = self
            .definitions
            .values()
            .flat_map(|biome| {
                biome.structures.iter().map(|structure| BiomeStructurePlacement {
                    biome_id: biome.id.clone(),
                    structure_id: structure.id.clone(),
                    placement: structure.placement,
                })
            })
            .collect();
        self.structure_placements.sort_by(|left, right| {
            left.biome_id
                .cmp(&right.biome_id)
                .then_with(|| left.structure_id.cmp(&right.structure_id))
        });
    }
}

fn default_biome_distributions() -> Vec<BiomeDistribution> {
    vec![BiomeDistribution::Regional]
}

fn default_vegetation_color() -> Hsi {
    Hsi::new(112.1111, 0.56363636, 0.36666667)
}
