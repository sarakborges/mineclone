use bevy::prelude::*;

pub const CHUNK_SIZE: usize = 16;
const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;
const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIZE;
const TEST_BLOCK_ID: &str = "mineclone:test_block";

#[derive(Component)]
pub struct VoxelChunk {
    blocks: [bool; CHUNK_VOLUME],
}

impl VoxelChunk {
    pub fn empty() -> Self {
        Self {
            blocks: [false; CHUNK_VOLUME],
        }
    }

    pub fn block_id_at(&self, x: i32, y: i32, z: i32) -> Option<&'static str> {
        self.is_solid(x, y, z).then_some(TEST_BLOCK_ID)
    }

    pub(crate) fn set_solid(&mut self, x: usize, y: usize, z: usize, solid: bool) {
        self.blocks[index(x, y, z)] = solid;
    }

    pub(crate) fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        if x < 0
            || y < 0
            || z < 0
            || x >= CHUNK_SIZE as i32
            || y >= CHUNK_SIZE as i32
            || z >= CHUNK_SIZE as i32
        {
            return false;
        }

        self.blocks[index(x as usize, y as usize, z as usize)]
    }
}

fn index(x: usize, y: usize, z: usize) -> usize {
    x + z * CHUNK_SIZE + y * CHUNK_AREA
}
