use bevy::prelude::*;

use crate::{
    content::fluid::FluidRegistry,
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid::FluidCell,
    },
    world::generation_region::GenerationRegion,
};

use super::index::voxel_index;

pub(super) fn rasterize_fluid_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    density: &[f32],
    fluids: &FluidRegistry,
    region: &GenerationRegion,
) {
    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.z + local_z as i32;
            let horizontal = Vec2::new(world_x as f32 + 0.5, world_z as f32 + 0.5);
            let Some(water) = region.hydrology.water_at(horizontal) else {
                continue;
            };
            let fluid_id = fluids.id_of(water.fluid_id).unwrap_or_else(|| {
                panic!("hydrology references missing fluid: {}", water.fluid_id)
            });

            for local_y in 0..CHUNK_SIZE {
                if density[voxel_index(local_x, local_y, local_z)] > 0.0 {
                    continue;
                }

                let world_y = chunk_origin.y + local_y as i32;
                if world_y as f32 >= water.water_level {
                    continue;
                }

                chunk.set_fluid(local_x, local_y, local_z, Some(FluidCell::source(fluid_id)));
            }
        }
    }
}
