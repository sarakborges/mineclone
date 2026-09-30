use bevy::prelude::*;

use super::{
    cell::VoxelCell, fluid::FluidCell, light::VoxelLight, object::ObjectCell, world::VoxelWorld,
};

pub(crate) trait VoxelRead: Send + Sync {
    fn sample_at(
        &self,
        world_position: IVec3,
    ) -> Option<(Option<VoxelCell>, Option<FluidCell>, VoxelLight)>;

    fn cell_at(&self, world_position: IVec3) -> Option<VoxelCell> {
        self.sample_at(world_position)
            .and_then(|(cell, _, _)| cell)
    }

    fn fluid_at(&self, world_position: IVec3) -> Option<FluidCell> {
        self.sample_at(world_position)
            .and_then(|(_, fluid, _)| fluid)
    }

    fn is_loaded_at(&self, world_position: IVec3) -> bool {
        self.sample_at(world_position).is_some()
    }

    fn is_solid(&self, world_position: IVec3) -> bool {
        self.cell_at(world_position).is_some()
    }

    fn block_id_at(&self, world_position: IVec3) -> Option<&'static str> {
        self.cell_at(world_position).map(|cell| cell.block_id)
    }
}

pub(crate) trait VoxelTopologyRead: VoxelRead {
    fn object_at(&self, world_position: IVec3) -> Option<ObjectCell>;
}

impl VoxelRead for VoxelWorld {
    fn sample_at(
        &self,
        world_position: IVec3,
    ) -> Option<(Option<VoxelCell>, Option<FluidCell>, VoxelLight)> {
        VoxelWorld::sample_at(self, world_position)
    }

    fn block_id_at(&self, world_position: IVec3) -> Option<&'static str> {
        VoxelWorld::block_id_at(self, world_position)
    }
}

impl VoxelTopologyRead for VoxelWorld {
    fn object_at(&self, world_position: IVec3) -> Option<ObjectCell> {
        VoxelWorld::object_at(self, world_position)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct VoxelTopologyReader<'a> {
    world: &'a VoxelWorld,
}

impl<'a> VoxelTopologyReader<'a> {
    pub(crate) fn new(world: &'a VoxelWorld) -> Self {
        Self { world }
    }
}

impl VoxelRead for VoxelTopologyReader<'_> {
    fn sample_at(
        &self,
        world_position: IVec3,
    ) -> Option<(Option<VoxelCell>, Option<FluidCell>, VoxelLight)> {
        self.world.sample_at(world_position)
    }
}

impl VoxelTopologyRead for VoxelTopologyReader<'_> {
    fn object_at(&self, world_position: IVec3) -> Option<ObjectCell> {
        self.world.object_at(world_position)
    }
}
