use bevy::prelude::*;

use super::{cell::VoxelCell, fluid::FluidCell, light::VoxelLight, world::VoxelWorld};

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

    #[cfg(test)]
    fn is_loaded_at(&self, world_position: IVec3) -> bool {
        self.sample_at(world_position).is_some()
    }

    fn block_id_at(&self, world_position: IVec3) -> Option<&'static str> {
        self.cell_at(world_position).map(|cell| cell.block_id)
    }
}

impl VoxelRead for VoxelWorld {
    fn sample_at(
        &self,
        world_position: IVec3,
    ) -> Option<(Option<VoxelCell>, Option<FluidCell>, VoxelLight)> {
        VoxelWorld::sample_at(self, world_position)
    }
}
