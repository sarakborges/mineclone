use crate::content::{biome::BiomeRegistry, structure::StructureRegistry};

pub(crate) fn clone_biome_registry(source: &BiomeRegistry) -> BiomeRegistry {
    let mut cloned = BiomeRegistry::default();
    for definition in source.iter() {
        cloned.insert(definition.clone());
    }
    cloned
}

pub(crate) fn clone_structure_registry(source: &StructureRegistry) -> StructureRegistry {
    let mut cloned = StructureRegistry::default();
    for definition in source.iter() {
        cloned.insert(definition.clone());
    }
    cloned
}
