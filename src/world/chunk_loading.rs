use bevy::prelude::IVec3;

use crate::voxel::world::VoxelWorld;

use super::generation::{ChunkGenerationContext, generate_chunk};

pub(super) fn ensure_chunk_loaded(
    world: &mut VoxelWorld,
    coord: IVec3,
    context: &ChunkGenerationContext<'_>,
) {
    if world.has_generated_chunk(coord) {
        assert!(
            world.restore_chunk(coord),
            "generated chunk must be resident or archived: {coord:?}"
        );
        return;
    }

    world.insert_chunk(coord, generate_chunk(coord, context));
}
