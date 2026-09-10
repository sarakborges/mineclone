use bevy::prelude::*;

use crate::{
    content::fluid::FluidRegistry,
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid::{FluidCell, MAX_FLUID_LEVEL},
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
                if world_y as f32 + 1.0 <= water.bed_level {
                    continue;
                }

                let Some(level) = fluid_level_for_surface(water.water_level, world_y) else {
                    continue;
                };

                chunk.set_fluid(
                    local_x,
                    local_y,
                    local_z,
                    Some(FluidCell::new(fluid_id, level)),
                );
            }
        }
    }
}

fn fluid_level_for_surface(water_level: f32, world_y: i32) -> Option<u8> {
    let coverage = water_level - world_y as f32;
    if coverage <= 0.0 {
        return None;
    }

    let level = (coverage.clamp(0.0, 1.0) * MAX_FLUID_LEVEL as f32).ceil() as u8;
    Some(level.clamp(1, MAX_FLUID_LEVEL))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_water_surface_produces_partial_fluid_level() {
        assert_eq!(fluid_level_for_surface(10.25, 10), Some(2));
        assert_eq!(fluid_level_for_surface(10.75, 10), Some(6));
        assert_eq!(fluid_level_for_surface(10.0, 9), Some(MAX_FLUID_LEVEL));
        assert_eq!(fluid_level_for_surface(10.0, 10), None);
    }
}
