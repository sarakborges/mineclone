use bevy::prelude::*;

use crate::{
    content::{
        structure::StructureRegistry,
        structure_set::StructureSetRegistry,
    },
    voxel::chunk::VoxelChunk,
    world::{biome_field::BiomeField, generation_region::GenerationRegion},
};

pub(super) fn rasterize_feature_pass(
    _chunk: &mut VoxelChunk,
    _chunk_origin: IVec3,
    _region: &GenerationRegion,
    _biome_field: &BiomeField,
    _structures: &StructureRegistry,
    _structure_sets: &StructureSetRegistry,
) {
    // Concrete decorations, structures, and structure sets are authored later.
    // Keeping them in the final feature pass preserves generation ordering while
    // allowing sets to resolve into ordinary structure placements before rasterization.
}
