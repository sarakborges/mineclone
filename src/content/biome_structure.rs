use serde::Deserialize;

use super::{biome::BiomeDefinition, structure::StructureRegistry};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructurePlacementRules {
    pub spacing: i32,
    pub chance: f32,
    #[serde(default)]
    pub jitter: i32,
}

impl StructurePlacementRules {
    fn validate(self, biome_id: &str, structure_id: &str) {
        assert!(
            self.spacing > 0,
            "biome {biome_id} structure {structure_id} placement spacing must be positive"
        );
        assert!(
            (0.0..=1.0).contains(&self.chance),
            "biome {biome_id} structure {structure_id} placement chance must be between 0 and 1"
        );
        assert!(
            self.jitter >= 0 && (self.jitter as i64) * 2 < self.spacing as i64,
            "biome {biome_id} structure {structure_id} placement jitter must be non-negative and smaller than half its spacing"
        );
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeStructure {
    pub id: String,
    pub placement: StructurePlacementRules,
}

impl BiomeDefinition {
    pub(crate) fn validate_structure_references(&self, structures: &StructureRegistry) {
        for (index, structure) in self.structures.iter().enumerate() {
            assert!(
                structures.get(&structure.id).is_some(),
                "biome {} references missing structure: {}",
                self.id,
                structure.id
            );
            assert!(
                !self.structures[..index]
                    .iter()
                    .any(|other| other.id == structure.id),
                "biome {} references structure {} more than once",
                self.id,
                structure.id
            );
            structure.placement.validate(&self.id, &structure.id);
        }
    }
}
