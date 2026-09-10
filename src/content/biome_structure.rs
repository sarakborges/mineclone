use super::{
    biome::BiomeDefinition,
    structure::StructureRegistry,
    structure_set::StructureSetRegistry,
};

impl BiomeDefinition {
    pub(crate) fn validate_structure_references(
        &self,
        structures: &StructureRegistry,
        structure_sets: &StructureSetRegistry,
    ) {
        for structure_id in &self.structures {
            assert!(
                structures.get(structure_id).is_some(),
                "biome {} references missing structure: {structure_id}",
                self.id
            );
        }

        for structure_set_id in &self.structure_sets {
            assert!(
                structure_sets.get(structure_set_id).is_some(),
                "biome {} references missing structure set: {structure_set_id}",
                self.id
            );
        }
    }
}
