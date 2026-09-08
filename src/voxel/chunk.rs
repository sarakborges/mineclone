use bevy::prelude::*;

use super::cell::VoxelCell;

pub const CHUNK_SIZE: usize = 16;
const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;
const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIZE;

#[derive(Component)]
pub struct VoxelChunk {
    blocks: [Option<VoxelCell>; CHUNK_VOLUME],
}

impl VoxelChunk {
    pub fn empty() -> Self {
        Self {
            blocks: [None; CHUNK_VOLUME],
        }
    }

    pub fn cell_at(&self, x: i32, y: i32, z: i32) -> Option<VoxelCell> {
        if x < 0
            || y < 0
            || z < 0
            || x >= CHUNK_SIZE as i32
            || y >= CHUNK_SIZE as i32
            || z >= CHUNK_SIZE as i32
        {
            return None;
        }

        self.blocks[index(x as usize, y as usize, z as usize)]
    }

    pub fn block_id_at(&self, x: i32, y: i32, z: i32) -> Option<&'static str> {
        self.cell_at(x, y, z).map(|cell| cell.block_id)
    }

    pub(crate) fn set_block(&mut self, x: usize, y: usize, z: usize, block: Option<VoxelCell>) {
        self.blocks[index(x, y, z)] = block;
    }

    pub(crate) fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        self.cell_at(x, y, z).is_some()
    }
}

fn index(x: usize, y: usize, z: usize) -> usize {
    x + z * CHUNK_SIZE + y * CHUNK_AREA
}
