use bevy::{ecs::system::SystemParam, prelude::*};

use crate::voxel::{
    lighting::{PendingLightingUpdates, process_pending_lighting},
    world::VoxelWorld,
};

use super::{chunk_remesh::ChunkRemeshQueue, chunk_system_params::VoxelContent};

const MAX_LIGHTING_VOXELS_PER_FRAME: usize = 4_096;

#[derive(SystemParam)]
pub(super) struct DynamicLightingRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
    lighting: ResMut<'w, PendingLightingUpdates>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
}

pub(super) fn has_pending_lighting_updates(lighting: Res<PendingLightingUpdates>) -> bool {
    !lighting.is_empty()
}

pub(super) fn process_dynamic_lighting(
    content: VoxelContent,
    mut runtime: DynamicLightingRuntime,
) {
    if runtime.lighting.is_empty() {
        return;
    }

    let changed_chunks = process_pending_lighting(
        &mut runtime.world,
        &mut runtime.lighting,
        &content.blocks,
        &content.fluids,
        &content.secondary_properties,
        MAX_LIGHTING_VOXELS_PER_FRAME,
    );

    for coord in changed_chunks {
        runtime.remesh_queue.enqueue_lighting_change(coord);
    }
}
