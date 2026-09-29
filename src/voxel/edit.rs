use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::layer::{LayerFace, LayerRegistry},
    world::{
        chunk_remesh::{ChunkRemeshQueue, prune_absent_remesh_halo},
        fluid_updates::PendingFluidUpdates,
    },
};

use super::{
    cell::VoxelCell,
    coordinates::ChunkCoord,
    fluid::FluidCell,
    layer::LayerCell,
    lighting::PendingLightingUpdates,
    object::ObjectCell,
    read::VoxelTopologyReader,
    world::VoxelWorld,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct VoxelBlockMutation {
    pub(crate) chunk: ChunkCoord,
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
    pub(crate) fn read(&self) -> VoxelTopologyReader<'_> {
        VoxelTopologyReader::new(&self.world)
    }

    pub(crate) fn cell_at(&self, world_position: IVec3) -> Option<VoxelCell> {
        self.world.cell_at(world_position)
    }

    pub(crate) fn add_layer(
        &mut self,
        world_position: IVec3,
        face: LayerFace,
        layer: LayerCell,
    ) -> Option<ChunkCoord> {
        let chunk = self
            .world
            .add_layer_at(world_position, face, layer, &self.layers)?;
        self.enqueue_voxel_remesh(world_position);
        Some(ChunkCoord::from_ivec3(chunk))
    }

    pub(crate) fn remove_top_layer(
        &mut self,
        world_position: IVec3,
        face: LayerFace,
    ) -> Option<&'static str> {
        let (_chunk, layer_id) = self.world.remove_top_layer_at(world_position, face)?;
        self.enqueue_voxel_remesh(world_position);
        Some(layer_id)
    }

    pub(crate) fn set_block(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<ChunkCoord> {
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
        self.enqueue_voxel_remesh(world_position);
        self.fluid_updates.enqueue_voxel_edit(world_position);
        Some(VoxelBlockMutation {
            chunk: ChunkCoord::from_ivec3(chunk),
            previous_cell,
            detached_object,
        })
    }

    pub(crate) fn set_fluid(
        &mut self,
        world_position: IVec3,
        fluid: Option<FluidCell>,
    ) -> Option<ChunkCoord> {
        let chunk = self.world.set_fluid_at(world_position, fluid)?;
        self.lighting.enqueue_medium_edit(world_position);
        self.enqueue_voxel_remesh(world_position);
        self.fluid_updates.enqueue_voxel_edit(world_position);
        Some(ChunkCoord::from_ivec3(chunk))
    }

    fn enqueue_voxel_remesh(&mut self, world_position: IVec3) {
        self.remesh_queue.enqueue_voxel_edit(world_position);
        prune_absent_remesh_halo(world_position, &self.world, &mut self.remesh_queue);
    }
}

#[derive(SystemParam)]
pub(crate) struct VoxelTopologyRuntime<'w> {
    mutation: VoxelMutationRuntime<'w>,
}

impl VoxelTopologyRuntime<'_> {
    pub(crate) fn read(&self) -> VoxelTopologyReader<'_> {
        self.mutation.read()
    }

    pub(crate) fn add_layer(
        &mut self,
        world_position: IVec3,
        face: LayerFace,
        layer: LayerCell,
    ) -> Option<IVec3> {
        self.mutation
            .add_layer(world_position, face, layer)
            .map(ChunkCoord::as_ivec3)
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
        self.mutation
            .set_block(world_position, block)
            .map(ChunkCoord::as_ivec3)
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
        self.mutation
            .set_fluid(world_position, fluid)
            .map(ChunkCoord::as_ivec3)
    }
}
