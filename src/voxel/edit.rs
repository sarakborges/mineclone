use bevy::{ecs::system::SystemParam, prelude::*};

use crate::world::{chunk_remesh::ChunkRemeshQueue, fluid_updates::PendingFluidUpdates};

use super::{cell::VoxelCell, lighting::PendingLightingUpdates, world::VoxelWorld};

#[derive(SystemParam)]
pub(crate) struct VoxelMutationRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
    lighting: ResMut<'w, PendingLightingUpdates>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
    fluid_updates: ResMut<'w, PendingFluidUpdates>,
}

impl VoxelMutationRuntime<'_> {
    pub(crate) fn world(&self) -> &VoxelWorld {
        &self.world
    }

    pub(crate) fn cell_at(&self, world_position: IVec3) -> Option<VoxelCell> {
        self.world.cell_at(world_position)
    }

    pub(crate) fn set_block(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<IVec3> {
        let (chunk, previous_cell) = self
            .world
            .set_block_at_with_previous(world_position, block)?;
        self.lighting
            .enqueue_voxel_edit(world_position, previous_cell);
        self.remesh_queue.enqueue_voxel_edit(chunk);
        self.fluid_updates.enqueue_voxel_edit(world_position);
        Some(chunk)
    }
}

#[derive(SystemParam)]
pub(crate) struct VoxelTopologyRuntime<'w> {
    mutation: VoxelMutationRuntime<'w>,
}

impl VoxelTopologyRuntime<'_> {
    pub(crate) fn world(&self) -> &VoxelWorld {
        self.mutation.world()
    }

    pub(crate) fn set_block(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<IVec3> {
        self.mutation.set_block(world_position, block)
    }
}
