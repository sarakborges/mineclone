use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, dimension::DimensionDefinition,
        structure::StructureRegistry, structure_set::StructureSetRegistry,
    },
    voxel::chunk::VoxelChunk,
    world::{biome_field::BiomeField, generation_region::GenerationRegion},
};

use super::structures::rasterize_structures;

pub(super) fn rasterize_feature_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    _region: &GenerationRegion,
    biome_field: &BiomeField,
    blocks: &BlockRegistry,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    structures: &StructureRegistry,
    _structure_sets: &StructureSetRegistry,
) {
    rasterize_structures(
        chunk,
        chunk_origin,
        dimension,
        biomes,
        blocks,
        biome_field,
        structures,
    );
}
