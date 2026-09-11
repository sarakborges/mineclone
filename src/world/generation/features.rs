use bevy::prelude::*;

use crate::voxel::chunk::VoxelChunk;

use super::{ChunkGenerationContext, structures::rasterize_structures};

pub(super) fn rasterize_feature_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    context: &ChunkGenerationContext<'_>,
) {
    rasterize_structures(chunk, chunk_origin, context);
}
