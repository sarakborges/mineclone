use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::{
    block::BlockRegistry, fluid::FluidRegistry,
    secondary_property::SecondaryPropertyRegistry,
};
use crate::voxel::{chunk::CHUNK_SIZE, light::VoxelLight, world::VoxelWorld};

use super::medium::medium_dampening_for_cells;

#[derive(Default)]
pub(super) struct LightingContext {
    direct_sky_columns: HashMap<IVec2, DirectSkyColumn>,
    highest_loaded_y_by_chunk_column: HashMap<IVec2, Option<i32>>,
}

struct DirectSkyColumn {
    highest_y: i32,
    levels_from_top: Vec<u8>,
}

impl DirectSkyColumn {
    fn new(highest_y: i32) -> Self {
        Self {
            highest_y,
            levels_from_top: Vec::new(),
        }
    }

    fn level_at(
        &mut self,
        world: &VoxelWorld,
        blocks: &BlockRegistry,
        fluids: &FluidRegistry,
        column: IVec2,
        y: i32,
    ) -> u8 {
        if y > self.highest_y {
            return VoxelLight::MAX_LEVEL;
        }

        let target_index = (self.highest_y - y) as usize;
        if target_index >= self.levels_from_top.len() {
            let mut level = self
                .levels_from_top
                .last()
                .copied()
                .unwrap_or(VoxelLight::MAX_LEVEL);
            let size = CHUNK_SIZE as i32;
            let chunk_x = column.x.div_euclid(size);
            let chunk_z = column.y.div_euclid(size);
            let local_x = column.x.rem_euclid(size);
            let local_z = column.y.rem_euclid(size);
            let mut next_y = self.highest_y - self.levels_from_top.len() as i32;

            while next_y >= y {
                let chunk_y = next_y.div_euclid(size);
                let segment_bottom = (chunk_y * size).max(y);
                let chunk_coord = IVec3::new(chunk_x, chunk_y, chunk_z);

                if let Some(chunk) = world.chunk(chunk_coord) {
                    for world_y in (segment_bottom..=next_y).rev() {
                        let local_y = world_y.rem_euclid(size);
                        let (cell, fluid, _) = chunk
                            .sample_local(local_x, local_y, local_z)
                            .expect("direct sky local coordinates must stay inside the chunk");
                        level = level.saturating_sub(medium_dampening_for_cells(
                            cell, fluid, blocks, fluids,
                        ));
                        self.levels_from_top.push(level);
                    }
                } else {
                    for _ in segment_bottom..=next_y {
                        self.levels_from_top.push(level);
                    }
                }

                next_y = segment_bottom - 1;
            }
        }

        self.levels_from_top[target_index]
    }
}

impl LightingContext {
    pub fn direct_sky_light(
        &mut self,
        world: &VoxelWorld,
        blocks: &BlockRegistry,
        fluids: &FluidRegistry,
        _secondary_properties: &SecondaryPropertyRegistry,
        position: IVec3,
    ) -> u8 {
        if position.y < 0 {
            return 0;
        }

        let column = IVec2::new(position.x, position.z);
        if let Some(direct_sky) = self.direct_sky_columns.get_mut(&column) {
            return direct_sky.level_at(world, blocks, fluids, column, position.y);
        }

        let Some(highest_y) = self.highest_loaded_y(world, position) else {
            return VoxelLight::MAX_LEVEL;
        };
        let mut direct_sky = DirectSkyColumn::new(highest_y);
        let level = direct_sky.level_at(world, blocks, fluids, column, position.y);
        self.direct_sky_columns.insert(column, direct_sky);
        level
    }

    fn highest_loaded_y(&mut self, world: &VoxelWorld, position: IVec3) -> Option<i32> {
        let size = CHUNK_SIZE as i32;
        let chunk_column = IVec2::new(position.x.div_euclid(size), position.z.div_euclid(size));

        if let Some(cached) = self.highest_loaded_y_by_chunk_column.get(&chunk_column) {
            return *cached;
        }

        let highest = world.highest_loaded_world_y_in_column(position.x, position.z);
        self.highest_loaded_y_by_chunk_column
            .insert(chunk_column, highest);
        highest
    }
}
