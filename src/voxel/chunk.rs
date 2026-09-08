use bevy::prelude::*;

use super::{cell::VoxelCell, fluid::FluidCell};

pub const CHUNK_SIZE: usize = 16;
const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;
const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIZE;

#[derive(Component)]
pub struct VoxelChunk {
    blocks: [Option<VoxelCell>; CHUNK_VOLUME],
    fluids: [Option<FluidCell>; CHUNK_VOLUME],
}

impl VoxelChunk {
    pub fn empty() -> Self {
        Self {
            blocks: [None; CHUNK_VOLUME],
            fluids: [None; CHUNK_VOLUME],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(Option::is_none) && self.fluids.iter().all(Option::is_none)
    }

    pub fn cell_at(&self, x: i32, y: i32, z: i32) -> Option<VoxelCell> {
        if !in_bounds(x, y, z) {
            return None;
        }

        self.blocks[index(x as usize, y as usize, z as usize)]
    }

    pub fn fluid_at(&self, x: i32, y: i32, z: i32) -> Option<FluidCell> {
        if !in_bounds(x, y, z) {
            return None;
        }

        self.fluids[index(x as usize, y as usize, z as usize)]
    }

    pub(crate) fn set_block(&mut self, x: usize, y: usize, z: usize, block: Option<VoxelCell>) {
        self.blocks[index(x, y, z)] = block;
    }

    pub(crate) fn set_fluid(&mut self, x: usize, y: usize, z: usize, fluid: Option<FluidCell>) {
        self.fluids[index(x, y, z)] = fluid;
    }
}

fn in_bounds(x: i32, y: i32, z: i32) -> bool {
    x >= 0
        && y >= 0
        && z >= 0
        && x < CHUNK_SIZE as i32
        && y < CHUNK_SIZE as i32
        && z < CHUNK_SIZE as i32
}

fn index(x: usize, y: usize, z: usize) -> usize {
    x + z * CHUNK_SIZE + y * CHUNK_AREA
}
