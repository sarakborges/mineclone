use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, dimension::DimensionDefinition,
        structure::StructureRegistry, structure_set::StructureSetRegistry,
    },
    voxel::chunk::VoxelChunk,
    world::{
        biome_field::BiomeField, cave_connectivity::CaveConnectivityRegion,
        generation_region::GenerationRegion,
    },
};

use super::structures::rasterize_structures;

pub(super) fn rasterize_feature_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    region: &GenerationRegion,
    anchored_caves: Option<&CaveConnectivityRegion>,
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
        region,
        anchored_caves,
        dimension,
        biomes,
        blocks,
        biome_field,
        structures,
    );
}
