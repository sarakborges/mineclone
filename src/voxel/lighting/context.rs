use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::{block::BlockRegistry, fluid::FluidRegistry};
use crate::voxel::{chunk::CHUNK_SIZE, light::VoxelLight, world::VoxelWorld};

use super::medium::medium_dampening;

#[derive(Default)]
pub(super) struct LightingContext {
    direct_sky_levels_by_column: HashMap<IVec2, Vec<u8>>,
    highest_loaded_y_by_chunk_column: HashMap<IVec2, Option<i32>>,
}

impl LightingContext {
    pub fn direct_sky_level(
        &mut self,
        world: &VoxelWorld,
        blocks: &BlockRegistry,
        fluids: &FluidRegistry,
        position: IVec3,
    ) -> u8 {
        if position.y < 0 {
            return 0;
        }

        let column = IVec2::new(position.x, position.z);
        if !self.direct_sky_levels_by_column.contains_key(&column) {
            let highest_loaded_y = self.highest_loaded_y(world, position);
            let levels = build_direct_sky_column(
                world,
                blocks,
                fluids,
                column,
                highest_loaded_y,
            );
            self.direct_sky_levels_by_column.insert(column, levels);
        }

        self.direct_sky_levels_by_column
            .get(&column)
            .and_then(|levels| levels.get(position.y as usize))
            .copied()
            .unwrap_or(VoxelLight::MAX_LEVEL)
    }

    fn highest_loaded_y(&mut self, world: &VoxelWorld, position: IVec3) -> Option<i32> {
        let size = CHUNK_SIZE as i32;
        let chunk_column = IVec2::new(
            position.x.div_euclid(size),
            position.z.div_euclid(size),
        );

        if let Some(cached) = self.highest_loaded_y_by_chunk_column.get(&chunk_column) {
            return *cached;
        }

        let highest = world.highest_loaded_world_y_in_column(position.x, position.z);
        self.highest_loaded_y_by_chunk_column
            .insert(chunk_column, highest);
        highest
    }
}

fn build_direct_sky_column(
    world: &VoxelWorld,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    column: IVec2,
    highest_y: Option<i32>,
) -> Vec<u8> {
    let Some(highest_y) = highest_y else {
        return Vec::new();
    };
    let mut levels = vec![0; highest_y as usize + 1];
    let mut level = VoxelLight::MAX_LEVEL;

    for y in (0..=highest_y).rev() {
        let position = IVec3::new(column.x, y, column.y);
        level = level.saturating_sub(medium_dampening(world, blocks, fluids, position));
        levels[y as usize] = level;
    }

    levels
}
