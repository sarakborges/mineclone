use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::layer::{LayerFace, LayerRegistry},
    world::{chunk_remesh::ChunkRemeshQueue, fluid_updates::PendingFluidUpdates},
};

use super::{
    cell::VoxelCell, fluid::FluidCell, layer::LayerCell, lighting::PendingLightingUpdates,
    object::ObjectCell, world::VoxelWorld,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct VoxelBlockMutation {
    pub(crate) chunk: IVec3,
    pub(crate) previous_cell: Option<VoxelCell>,
    pub(crate) detached_object: Option<ObjectCell>,
}

#[derive(SystemParam)]
pub(crate) struct VoxelMutationRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
    layers: Res<'w, LayerRegistry>,
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

    pub(crate) fn add_layer(
        &mut self,
        world_position: IVec3,
        face: LayerFace,
        layer: LayerCell,
    ) -> Option<IVec3> {
        let chunk = self
            .world
            .add_layer_at(world_position, face, layer, &self.layers)?;
        self.remesh_queue.enqueue_voxel_edit(world_position);
        Some(chunk)
    }

    pub(crate) fn remove_top_layer(
        &mut self,
        world_position: IVec3,
        face: LayerFace,
    ) -> Option<&'static str> {
        let (_chunk, layer_id) = self.world.remove_top_layer_at(world_position, face)?;
        self.remesh_queue.enqueue_voxel_edit(world_position);
        Some(layer_id)
    }

    pub(crate) fn set_block(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<IVec3> {
        self.set_block_detailed(world_position, block)
            .map(|mutation| mutation.chunk)
    }

    pub(crate) fn set_block_detailed(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<VoxelBlockMutation> {
        let (chunk, previous_cell, detached_object) = self
            .world
            .set_block_at_with_previous(world_position, block)?;
        self.lighting
            .enqueue_voxel_edit(world_position, previous_cell);
        self.remesh_queue.enqueue_voxel_edit(world_position);
        self.fluid_updates.enqueue_voxel_edit(world_position);
        Some(VoxelBlockMutation {
            chunk,
            previous_cell,
            detached_object,
        })
    }

    pub(crate) fn set_fluid(
        &mut self,
        world_position: IVec3,
        fluid: Option<FluidCell>,
    ) -> Option<IVec3> {
        let chunk = self.world.set_fluid_at(world_position, fluid)?;
        self.lighting.enqueue_medium_edit(world_position);
        self.remesh_queue.enqueue_voxel_edit(world_position);
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

    pub(crate) fn add_layer(
        &mut self,
        world_position: IVec3,
        face: LayerFace,
        layer: LayerCell,
    ) -> Option<IVec3> {
        self.mutation.add_layer(world_position, face, layer)
    }

    pub(crate) fn remove_top_layer(
        &mut self,
        world_position: IVec3,
        face: LayerFace,
    ) -> Option<&'static str> {
        self.mutation.remove_top_layer(world_position, face)
    }

    pub(crate) fn set_block(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<IVec3> {
        self.mutation.set_block(world_position, block)
    }

    pub(crate) fn set_block_detailed(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<VoxelBlockMutation> {
        self.mutation.set_block_detailed(world_position, block)
    }

    pub(crate) fn set_fluid(
        &mut self,
        world_position: IVec3,
        fluid: Option<FluidCell>,
    ) -> Option<IVec3> {
        self.mutation.set_fluid(world_position, fluid)
    }
}
