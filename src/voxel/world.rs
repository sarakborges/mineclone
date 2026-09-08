use std::collections::HashMap;

use bevy::prelude::*;

use super::chunk::{VoxelChunk, CHUNK_SIZE};

#[derive(Resource, Default)]
pub struct VoxelWorld {
    chunks: HashMap<IVec2, VoxelChunk>,
}

impl VoxelWorld {
    pub fn insert_chunk(&mut self, coord: IVec2, chunk: VoxelChunk) {
        self.chunks.insert(coord, chunk);
    }

    pub fn chunks(&self) -> impl Iterator<Item = (&IVec2, &VoxelChunk)> {
        self.chunks.iter()
    }

    pub fn is_solid(&self, world_position: IVec3) -> bool {
        let Some((chunk_coord, local_position)) = split_world_position(world_position) else {
            return false;
        };

        self.chunks
            .get(&chunk_coord)
            .is_some_and(|chunk| chunk.is_solid(local_position.x, local_position.y, local_position.z))
    }

    pub fn block_id_at(&self, world_position: IVec3) -> Option<&'static str> {
        let (chunk_coord, local_position) = split_world_position(world_position)?;

        self.chunks.get(&chunk_coord)?.block_id_at(
            local_position.x,
            local_position.y,
            local_position.z,
        )
    }
}

fn split_world_position(world_position: IVec3) -> Option<(IVec2, IVec3)> {
    if world_position.y < 0 || world_position.y >= CHUNK_SIZE as i32 {
        return None;
    }

    let chunk_size = CHUNK_SIZE as i32;
    let chunk_coord = IVec2::new(
        world_position.x.div_euclid(chunk_size),
        world_position.z.div_euclid(chunk_size),
    );
    let local_position = IVec3::new(
        world_position.x.rem_euclid(chunk_size),
        world_position.y,
        world_position.z.rem_euclid(chunk_size),
    );

    Some((chunk_coord, local_position))
}
