use bevy::prelude::*;

use crate::voxel::{
    lighting::{PendingLightingUpdates, process_pending_lighting},
    world::VoxelWorld,
};

use super::{chunk_remesh::ChunkRemeshQueue, chunk_system_params::ChunkContent};

const MAX_LIGHTING_VOXELS_PER_FRAME: usize = 8_192;

pub(super) fn process_dynamic_lighting(
    content: ChunkContent,
    mut world: ResMut<VoxelWorld>,
    mut lighting: ResMut<PendingLightingUpdates>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
) {
    let immediate_refresh = remesh_queue.has_immediate_lighting();
    let changed_chunks = process_pending_lighting(
        &mut world,
        &mut lighting,
        &content.blocks,
        &content.fluids,
        &content.secondary_properties,
        MAX_LIGHTING_VOXELS_PER_FRAME,
    );

    for coord in changed_chunks {
        if immediate_refresh {
            remesh_queue.enqueue_lighting_change(coord);
        } else {
            remesh_queue.enqueue_priority(coord);
            remesh_queue.enqueue_voxel_edit_neighbors(coord);
        }
    }
}

pub(super) fn clear_dynamic_lighting(mut lighting: ResMut<PendingLightingUpdates>) {
    lighting.clear();
}
