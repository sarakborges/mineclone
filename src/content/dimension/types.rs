use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use crate::content::registry::DefinitionMap;

const MAX_GENERATED_SURFACE_FLUID_SPACING: u32 = 512;
const MAX_GENERATED_SURFACE_FLUID_RADIUS: u32 = 256;
const MAX_GENERATED_SURFACE_FLUID_DEPTH: u32 = 4;
const DEFAULT_GENERATED_SURFACE_FLUID_CHANCE: f32 = 1.0;
const DEFAULT_GENERATED_SURFACE_FLUID_DEPTH: u32 = 1;
const MAX_GENERATED_SURFACE_STRUCTURE_SPACING: u32 = 16_384;
const DEFAULT_GENERATED_SURFACE_STRUCTURE_CHANCE: f32 = 1.0;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedOceanDefinition {
    pub biome: String,
    pub fluid: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedSurfaceFluidDefinition {
    pub biome: String,
    pub fluid: String,
    pub spacing: u32,
    pub radius: u32,
    #[serde(default)]
    pub jitter: u32,
    #[serde(default = "default_generated_surface_fluid_chance")]
    pub chance: f32,
    #[serde(default = "default_generated_surface_fluid_depth")]
    pub depth: u32,
}

/// One deterministic world-space root placement rule for a surface Structure
/// or Structure group. Internal StructureSet and connector expansion remain
/// Structure concerns and are resolved by the generation-side Structure owner.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedSurfaceStructureDefinition {
    pub biome: String,
    pub structure: String,
    pub spacing: u32,
    #[serde(default)]
    pub jitter: u32,
    #[serde(default = "default_generated_surface_structure_chance")]
    pub chance: f32,
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
    #[serde(default)]
    pub generated_surface_fluids: Vec<GeneratedSurfaceFluidDefinition>,
    #[serde(default)]
    pub generated_surface_structures: Vec<GeneratedSurfaceStructureDefinition>,
}

fn default_max_entities() -> usize {
    128
}

fn default_generated_surface_fluid_chance() -> f32 {
    DEFAULT_GENERATED_SURFACE_FLUID_CHANCE
}

fn default_generated_surface_fluid_depth() -> u32 {
    DEFAULT_GENERATED_SURFACE_FLUID_DEPTH
}

fn default_generated_surface_structure_chance() -> f32 {
    DEFAULT_GENERATED_SURFACE_STRUCTURE_CHANCE
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
        for (index, surface_fluid) in definition.generated_surface_fluids.iter().enumerate() {
            validate_generated_surface_fluid(&definition.id, index, surface_fluid);
            assert!(
                !definition.generated_surface_fluids[..index]
                    .iter()
                    .any(|previous| previous.biome == surface_fluid.biome),
                "dimension {} generatedSurfaceFluids cannot define more than one local surface fluid rule for biome {}",
                definition.id,
                surface_fluid.biome
            );
        }
        for (index, structure) in definition.generated_surface_structures.iter().enumerate() {
            validate_generated_surface_structure(&definition.id, index, structure);
            assert!(
                !definition.generated_surface_structures[..index]
                    .iter()
                    .any(|previous| {
                        previous.biome == structure.biome
                            && previous.structure == structure.structure
                    }),
                "dimension {} generatedSurfaceStructures cannot repeat structure {} for biome {}",
                definition.id,
                structure.structure,
                structure.biome
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

fn validate_generated_surface_fluid(
    dimension_id: &str,
    index: usize,
    definition: &GeneratedSurfaceFluidDefinition,
) {
    assert_namespaced_id(
        dimension_id,
        &format!("generatedSurfaceFluids[{index}].biome"),
        &definition.biome,
    );
    assert_namespaced_id(
        dimension_id,
        &format!("generatedSurfaceFluids[{index}].fluid"),
        &definition.fluid,
    );
    assert!(
        (2..=MAX_GENERATED_SURFACE_FLUID_SPACING).contains(&definition.spacing),
        "dimension {dimension_id} generatedSurfaceFluids[{index}].spacing must be within 2..={MAX_GENERATED_SURFACE_FLUID_SPACING}"
    );
    assert!(
        definition.radius > 0 && definition.radius <= MAX_GENERATED_SURFACE_FLUID_RADIUS,
        "dimension {dimension_id} generatedSurfaceFluids[{index}].radius must be within 1..={MAX_GENERATED_SURFACE_FLUID_RADIUS}"
    );
    assert!(
        definition.jitter <= definition.spacing / 2,
        "dimension {dimension_id} generatedSurfaceFluids[{index}].jitter must not exceed half the spacing"
    );
    assert!(
        definition.radius.saturating_add(definition.jitter) <= definition.spacing,
        "dimension {dimension_id} generatedSurfaceFluids[{index}] radius + jitter must not exceed spacing"
    );
    assert!(
        definition.chance.is_finite() && definition.chance > 0.0 && definition.chance <= 1.0,
        "dimension {dimension_id} generatedSurfaceFluids[{index}].chance must be finite and within (0, 1]"
    );
    assert!(
        (1..=MAX_GENERATED_SURFACE_FLUID_DEPTH).contains(&definition.depth),
        "dimension {dimension_id} generatedSurfaceFluids[{index}].depth must be within 1..={MAX_GENERATED_SURFACE_FLUID_DEPTH}"
    );
}

fn validate_generated_surface_structure(
    dimension_id: &str,
    index: usize,
    definition: &GeneratedSurfaceStructureDefinition,
) {
    assert_namespaced_id(
        dimension_id,
        &format!("generatedSurfaceStructures[{index}].biome"),
        &definition.biome,
    );
    assert_namespaced_id(
        dimension_id,
        &format!("generatedSurfaceStructures[{index}].structure"),
        &definition.structure,
    );
    assert!(
        (2..=MAX_GENERATED_SURFACE_STRUCTURE_SPACING).contains(&definition.spacing),
        "dimension {dimension_id} generatedSurfaceStructures[{index}].spacing must be within 2..={MAX_GENERATED_SURFACE_STRUCTURE_SPACING}"
    );
    assert!(
        definition.jitter.saturating_mul(2) < definition.spacing,
        "dimension {dimension_id} generatedSurfaceStructures[{index}].jitter must be smaller than half the spacing"
    );
    assert!(
        definition.chance.is_finite() && (0.0..=1.0).contains(&definition.chance),
        "dimension {dimension_id} generatedSurfaceStructures[{index}].chance must be finite and within [0, 1]"
    );
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
