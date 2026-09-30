use std::sync::Arc;

use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use self::validation::validate_biome_definition;
use super::{
    biome_density::BiomeDensityModifier, biome_distribution::BiomeDistribution,
    biome_material::BiomeMaterialLayer,
    biome_sky_layer::BiomeSkyLayerVisuals,
    biome_structure::{BiomeStructure, BiomeStructurePlacementRules},
    biome_surface_fluid::BiomeSurfaceFluid,
    biome_surface_margin::BiomeSurfaceMargin, biome_terrain::BiomeTerrain,
    biome_terrain_modifier::BiomeTerrainModifier, color::Hsi, creature::CreatureRegistry,
    day_night_phase::DayNightPhases, fluid::FluidRegistry, object::ObjectRegistry,
    block::BlockRegistry, registry::DefinitionMap,
};

mod validation;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BiomeKind {
    #[default]
    Surface,
    Volume,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeObjectSpawnRule {
    pub object: String,
    pub spacing: i32,
    pub chance: f32,
    #[serde(default)]
    pub jitter: i32,
    #[serde(default = "default_object_cluster_min")]
    pub cluster_min: u8,
    #[serde(default = "default_object_cluster_max")]
    pub cluster_max: u8,
    #[serde(default)]
    pub cluster_radius: i32,
    #[serde(default)]
    pub ground_blocks: Vec<String>,
}

fn default_object_cluster_min() -> u8 { 1 }
fn default_object_cluster_max() -> u8 { 1 }

impl BiomeObjectSpawnRule {
    fn validate(&self, biome_id: &str) {
        assert!(self.spacing > 0, "biome {biome_id} object spawn {} spacing must be positive", self.object);
        assert!(
            self.chance.is_finite() && (0.0..=1.0).contains(&self.chance),
            "biome {biome_id} object spawn {} chance must be between 0 and 1",
            self.object
        );
        assert!(
            self.jitter >= 0 && (self.jitter as i64) * 2 < self.spacing as i64,
            "biome {biome_id} object spawn {} jitter must be smaller than half its spacing",
            self.object
        );
        assert!(
            self.cluster_min > 0 && self.cluster_max >= self.cluster_min,
            "biome {biome_id} object spawn {} cluster range is invalid",
            self.object
        );
        assert!(
            self.cluster_radius >= 0 && self.cluster_radius < self.spacing,
            "biome {biome_id} object spawn {} clusterRadius must be non-negative and smaller than spacing",
            self.object
        );
    }
}

fn default_spawn_weight() -> f32 { 1.0 }
fn default_spawn_light_max() -> u8 { 15 }
fn default_spawn_spacing() -> f32 { 16.0 }

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceBiomeSelector {
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl SurfaceBiomeSelector {
    pub(crate) fn matches(&self, surface_id: &str, surface_tags: &[String]) -> bool {
        self.ids.iter().any(|id| id == surface_id)
            || self
                .tags
                .iter()
                .any(|tag| surface_tags.iter().any(|candidate| candidate == tag))
    }

    pub(crate) fn validate(&self, biome_id: &str, field: &str) {
        assert!(
            !self.ids.is_empty() || !self.tags.is_empty(),
            "biome {biome_id} {field} must define at least one id or tag"
        );
        for (index, id) in self.ids.iter().enumerate() {
            assert!(
                !id.trim().is_empty(),
                "biome {biome_id} {field}.ids[{index}] cannot be empty"
            );
        }
        for (index, tag) in self.tags.iter().enumerate() {
            assert!(
                !tag.trim().is_empty(),
                "biome {biome_id} {field}.tags[{index}] cannot be empty"
            );
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSurfaceConstraints {
    #[serde(default)]
    pub allow: Option<SurfaceBiomeSelector>,
    #[serde(default)]
    pub deny: Option<SurfaceBiomeSelector>,
}

impl VolumeSurfaceConstraints {
    pub(crate) fn allows_surface(&self, surface_id: &str, surface_tags: &[String]) -> bool {
        if self
            .deny
            .as_ref()
            .is_some_and(|selector| selector.matches(surface_id, surface_tags))
        {
            return false;
        }

        self.allow
            .as_ref()
            .is_none_or(|selector| selector.matches(surface_id, surface_tags))
    }

    pub(crate) fn validate(&self, biome_id: &str) {
        if let Some(allow) = &self.allow {
            allow.validate(biome_id, "surfaceConstraints.allow");
        }
        if let Some(deny) = &self.deny {
            deny.validate(biome_id, "surfaceConstraints.deny");
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeDefinition {
    pub id: String,
    pub name: LocalizedText,
    #[serde(default)]
    pub kind: BiomeKind,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub surface_constraints: Option<VolumeSurfaceConstraints>,
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
    pub surface_fluid: Option<BiomeSurfaceFluid>,
    #[serde(default)]
    pub surface_margin: Option<BiomeSurfaceMargin>,
    #[serde(default)]
    pub surface_layers: Vec<BiomeMaterialLayer>,
    #[serde(default)]
    pub density_modifier: Option<BiomeDensityModifier>,
    #[serde(default)]
    pub solid_block: Option<String>,
    #[serde(default)]
    pub structures: Vec<BiomeStructure>,
    #[serde(default)]
    pub creature_spawns: Vec<CreatureSpawnRule>,
    #[serde(default)]
    pub object_spawns: Vec<BiomeObjectSpawnRule>,
    #[serde(default)]
    pub visuals: Option<BiomeVisuals>,
}

impl BiomeDefinition {
    pub fn visuals(&self) -> &BiomeVisuals {
        self.visuals
            .as_ref()
            .unwrap_or_else(|| panic!("biome {} does not define visuals", self.id))
    }

    pub(crate) fn validate_spawn_references(&self, creatures: &CreatureRegistry) {
        for spawn in &self.creature_spawns {
            assert!(
                creatures.get(&spawn.creature).is_some(),
                "biome {} references missing creature spawn: {}",
                self.id,
                spawn.creature
            );
        }
    }

    pub(crate) fn validate_object_spawn_references(
        &self,
        objects: &ObjectRegistry,
        blocks: &BlockRegistry,
    ) {
        for spawn in &self.object_spawns {
            spawn.validate(&self.id);
            assert!(
                objects.get(&spawn.object).is_some(),
                "biome {} references missing object spawn: {}",
                self.id,
                spawn.object
            );
            for block in &spawn.ground_blocks {
                assert!(
                    blocks.get(block).is_some(),
                    "biome {} object spawn {} references missing ground block: {}",
                    self.id,
                    spawn.object,
                    block
                );
            }
        }
    }

    pub(crate) fn validate_surface_fluid_references(&self, fluids: &FluidRegistry) {
        if let Some(surface_fluid) = &self.surface_fluid {
            assert!(
                fluids.id_of(surface_fluid.fluid_id()).is_some(),
                "biome {} surfaceFluid references missing fluid {}",
                self.id,
                surface_fluid.fluid_id()
            );
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BiomeStructurePlacement {
    pub(crate) biome_id: String,
    pub(crate) structure_id: String,
    pub(crate) placement: BiomeStructurePlacementRules,
}

#[derive(Clone, Resource, Default)]
pub struct BiomeRegistry {
    definitions: DefinitionMap<BiomeDefinition>,
    has_volume_density_modifiers: bool,
    has_volume_solid_density_modifiers: bool,
    structure_placements: Arc<Vec<BiomeStructurePlacement>>,
}

impl BiomeRegistry {
    pub fn insert(&mut self, definition: BiomeDefinition) {
        assert!(!definition.id.trim().is_empty(), "biome id cannot be empty");
        definition
            .name
            .validate(&format!("biome {} name", definition.id));
        validate_biome_definition(&definition);

        let has_density_modifier =
            definition.kind == BiomeKind::Volume && definition.density_modifier.is_some();
        let has_solid_density_modifier = definition.kind == BiomeKind::Volume
            && matches!(
                definition.density_modifier,
                Some(BiomeDensityModifier::Solid { .. })
            );
        let biome_id = definition.id.clone();
        let placements = definition
            .structures
            .iter()
            .map(|structure| BiomeStructurePlacement {
                biome_id: biome_id.clone(),
                structure_id: structure.id.clone(),
                placement: structure.placement_for_biome(&biome_id, definition.kind),
            })
            .collect::<Vec<_>>();

        self.definitions.insert(biome_id, definition);
        self.has_volume_density_modifiers |= has_density_modifier;
        self.has_volume_solid_density_modifiers |= has_solid_density_modifier;
        let structure_placements = Arc::make_mut(&mut self.structure_placements);
        structure_placements.extend(placements);
        structure_placements.sort_by(|left, right| {
            left.biome_id
                .cmp(&right.biome_id)
                .then_with(|| left.structure_id.cmp(&right.structure_id))
        });
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
}

fn default_biome_distributions() -> Vec<BiomeDistribution> {
    vec![BiomeDistribution::Regional]
}

fn default_vegetation_color() -> Hsi {
    Hsi::new(112.1111, 0.56363636, 0.36666667)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn surface_selector_matches_ids_or_tags() {
        let selector = SurfaceBiomeSelector {
            ids: vec!["asteria:overworld/plains".to_string()],
            tags: vec!["mountain".to_string()],
        };

        assert!(selector.matches("asteria:overworld/plains", &[]));
        assert!(selector.matches("asteria:overworld/alps", &tags(&["mountain", "cold"])));
        assert!(!selector.matches("asteria:overworld/ocean", &tags(&["water"])));
    }

    #[test]
    fn volume_surface_constraints_default_to_allow_and_deny_wins() {
        let unrestricted = VolumeSurfaceConstraints::default();
        assert!(unrestricted.allows_surface("asteria:overworld/ocean", &tags(&["water"])));

        let constrained = VolumeSurfaceConstraints {
            allow: Some(SurfaceBiomeSelector {
                ids: Vec::new(),
                tags: vec!["mountain".to_string()],
            }),
            deny: Some(SurfaceBiomeSelector {
                ids: vec!["asteria:overworld/volcano".to_string()],
                tags: Vec::new(),
            }),
        };

        assert!(constrained.allows_surface(
            "asteria:overworld/alps",
            &tags(&["mountain"]),
        ));
        assert!(!constrained.allows_surface(
            "asteria:overworld/volcano",
            &tags(&["mountain"]),
        ));
        assert!(!constrained.allows_surface("asteria:overworld/plains", &[]));
    }
}
