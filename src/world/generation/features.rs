use bevy::prelude::*;

use crate::{
    voxel::chunk::VoxelChunk,
    world::{biome_field::BiomeField, generation_region::GenerationRegion},
};

pub(super) fn rasterize_feature_pass(
    _chunk: &mut VoxelChunk,
    _chunk_origin: IVec3,
    _region: &GenerationRegion,
    _biome_field: &BiomeField,
) {
    // Concrete trees, crystals, roots, structures, and similar content are
    // authored later. Keeping the pass explicit preserves generation ordering.
}
