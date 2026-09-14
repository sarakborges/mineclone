use bevy::prelude::*;

use super::{cell::VoxelCell, fluid::FluidCell, light::VoxelLight, world::VoxelWorld};

pub(crate) trait VoxelRead: Send + Sync {
    fn cell_at(&self, world_position: IVec3) -> Option<VoxelCell>;
    fn fluid_at(&self, world_position: IVec3) -> Option<FluidCell>;
    fn light_at(&self, world_position: IVec3) -> VoxelLight;
    fn is_loaded_at(&self, world_position: IVec3) -> bool;

    fn is_solid(&self, world_position: IVec3) -> bool {
        self.cell_at(world_position).is_some()
    }

    fn block_id_at(&self, world_position: IVec3) -> Option<&'static str> {
        self.cell_at(world_position).map(|cell| cell.block_id)
    }
}

impl VoxelRead for VoxelWorld {
    fn cell_at(&self, world_position: IVec3) -> Option<VoxelCell> {
        VoxelWorld::cell_at(self, world_position)
    }

    fn fluid_at(&self, world_position: IVec3) -> Option<FluidCell> {
        VoxelWorld::fluid_at(self, world_position)
    }

    fn light_at(&self, world_position: IVec3) -> VoxelLight {
        VoxelWorld::light_at(self, world_position)
    }

    fn is_loaded_at(&self, world_position: IVec3) -> bool {
        VoxelWorld::is_loaded_at(self, world_position)
    }
}
