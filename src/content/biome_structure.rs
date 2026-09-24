use serde::Deserialize;

use super::{
    biome::{BiomeDefinition, BiomeKind},
    structure::StructureRegistry,
    structure_set::StructureSetRegistry,
};

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

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeStructurePlacementRules {
    pub chance: f32,
}

impl VolumeStructurePlacementRules {
    fn validate(self, biome_id: &str, structure_id: &str) {
        assert!(
            self.chance.is_finite() && (0.0..=1.0).contains(&self.chance),
            "volume biome {biome_id} structure {structure_id} volumePlacement chance must be between 0 and 1"
        );
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum BiomeStructurePlacementRules {
    Surface(StructurePlacementRules),
    Volume(VolumeStructurePlacementRules),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeStructure {
    pub id: String,
    #[serde(default)]
    pub placement: Option<StructurePlacementRules>,
    #[serde(default)]
    pub volume_placement: Option<VolumeStructurePlacementRules>,
}

impl BiomeStructure {
    pub(crate) fn placement_for_biome(
        &self,
        biome_id: &str,
        kind: BiomeKind,
    ) -> BiomeStructurePlacementRules {
        match kind {
            BiomeKind::Surface => {
                assert!(
                    self.volume_placement.is_none(),
                    "surface biome {biome_id} structure {} cannot define volumePlacement",
                    self.id
                );
                let placement = self.placement.unwrap_or_else(|| {
                    panic!(
                        "surface biome {biome_id} structure {} must define placement",
                        self.id
                    )
                });
                placement.validate(biome_id, &self.id);
                BiomeStructurePlacementRules::Surface(placement)
            }
            BiomeKind::Volume => {
                assert!(
                    self.placement.is_none(),
                    "volume biome {biome_id} structure {} cannot define surface placement",
                    self.id
                );
                let placement = self.volume_placement.unwrap_or_else(|| {
                    panic!(
                        "volume biome {biome_id} structure {} must define volumePlacement",
                        self.id
                    )
                });
                placement.validate(biome_id, &self.id);
                BiomeStructurePlacementRules::Volume(placement)
            }
        }
    }
}

impl BiomeDefinition {
    pub(crate) fn validate_structure_references(
        &self,
        structures: &StructureRegistry,
        structure_sets: &StructureSetRegistry,
    ) {
        for (index, structure) in self.structures.iter().enumerate() {
            assert!(
                structures.resolves_reference(&structure.id)
                    || structure_sets.get(&structure.id).is_some(),
                "biome {} references missing structure, structure group, or structure set: {}",
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
            let placement = structure.placement_for_biome(&self.id, self.kind);
            if matches!(placement, BiomeStructurePlacementRules::Volume(_)) {
                assert!(
                    structure_sets.get(&structure.id).is_none(),
                    "volume biome {} structure {} must reference a Structure or Structure Group, not a Structure Set",
                    self.id,
                    structure.id
                );
                assert!(
                    self.vertical_range.is_some(),
                    "volume biome {} with structures must define verticalRange",
                    self.id
                );
            }
        }
    }
}
