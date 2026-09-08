use std::collections::HashMap;

use bevy::prelude::*;

use super::{
    cell::VoxelCell,
    chunk::{VoxelChunk, CHUNK_SIZE},
};

#[derive(Resource, Default)]
pub struct VoxelWorld {
    chunks: HashMap<IVec3, VoxelChunk>,
}

impl VoxelWorld {
    pub fn insert_chunk(&mut self, coord: IVec3, chunk: VoxelChunk) {
        self.chunks.insert(coord, chunk);
    }

    pub fn chunk(&self, coord: IVec3) -> Option<&VoxelChunk> {
        self.chunks.get(&coord)
    }

    pub fn chunks(&self) -> impl Iterator<Item = (&IVec3, &VoxelChunk)> {
        self.chunks.iter()
    }

    pub fn cell_at(&self, world_position: IVec3) -> Option<VoxelCell> {
        let (chunk_coord, local_position) = split_world_position(world_position);

        self.chunks.get(&chunk_coord)?.cell_at(
            local_position.x,
            local_position.y,
            local_position.z,
        )
    }

    pub fn is_solid(&self, world_position: IVec3) -> bool {
        self.cell_at(world_position).is_some()
    }

    pub fn block_id_at(&self, world_position: IVec3) -> Option<&'static str> {
        self.cell_at(world_position).map(|cell| cell.block_id)
    }
}

fn split_world_position(world_position: IVec3) -> (IVec3, IVec3) {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_coord = IVec3::new(
        world_position.x.div_euclid(chunk_size),
        world_position.y.div_euclid(chunk_size),
        world_position.z.div_euclid(chunk_size),
    );
    let local_position = IVec3::new(
        world_position.x.rem_euclid(chunk_size),
        world_position.y.rem_euclid(chunk_size),
        world_position.z.rem_euclid(chunk_size),
    );

    (chunk_coord, local_position)
}
