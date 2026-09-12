use bevy::prelude::*;

use crate::voxel::{
    lighting::{PendingLightingUpdates, process_pending_lighting},
    world::VoxelWorld,
};

use super::{chunk_remesh::ChunkRemeshQueue, chunk_system_params::ChunkContent};

const MAX_LIGHTING_VOXELS_PER_FRAME: usize = 4096;

pub(super) fn process_dynamic_lighting(
    content: ChunkContent,
    mut world: ResMut<VoxelWorld>,
    mut lighting: ResMut<PendingLightingUpdates>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
) {
    let changed_chunks = process_pending_lighting(
        &mut world,
        &mut lighting,
        &content.blocks,
        &content.fluids,
        &content.secondary_properties,
        MAX_LIGHTING_VOXELS_PER_FRAME,
    );

    remesh_queue.extend(changed_chunks);
}

pub(super) fn clear_dynamic_lighting(mut lighting: ResMut<PendingLightingUpdates>) {
    lighting.clear();
}
