use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use super::{
    cell::VoxelCell,
    chunk::{VoxelChunk, CHUNK_SIZE},
    chunk_archive::ArchivedChunk,
    fluid::FluidCell,
    skylight::relight_chunk_and_neighbors,
};

#[derive(Resource, Default)]
pub struct VoxelWorld {
    chunks: HashMap<IVec3, VoxelChunk>,
    archived_chunks: HashMap<IVec3, ArchivedChunk>,
    generated_chunks: HashSet<IVec3>,
}

impl VoxelWorld {
    pub fn insert_chunk(&mut self, coord: IVec3, chunk: VoxelChunk) {
        assert!(coord.y >= 0, "chunk Y cannot be negative: {}", coord.y);
        assert!(
            !self.generated_chunks.contains(&coord),
            "worldgen cannot overwrite an already generated chunk: {coord:?}"
        );

        self.generated_chunks.insert(coord);
        self.chunks.insert(coord, chunk);
        relight_chunk_and_neighbors(self, coord);
    }

    pub fn chunk(&self, coord: IVec3) -> Option<&VoxelChunk> {
        if coord.y < 0 {
            return None;
        }

        self.chunks.get(&coord)
    }

    pub(crate) fn chunk_mut(&mut self, coord: IVec3) -> Option<&mut VoxelChunk> {
        if coord.y < 0 {
            return None;
        }

        self.chunks.get_mut(&coord)
    }

    pub fn archive_chunk(&mut self, coord: IVec3) {
        let Some(chunk) = self.chunks.remove(&coord) else {
            return;
        };

        self.archived_chunks
            .insert(coord, ArchivedChunk::from_chunk(&chunk));
    }

    pub fn restore_chunk(&mut self, coord: IVec3) -> bool {
        if self.chunks.contains_key(&coord) {
            return true;
        }

        let Some(archived) = self.archived_chunks.remove(&coord) else {
            return false;
        };

        self.chunks.insert(coord, archived.restore());
        relight_chunk_and_neighbors(self, coord);
        true
    }

    pub fn has_generated_chunk(&self, coord: IVec3) -> bool {
        coord.y >= 0 && self.generated_chunks.contains(&coord)
    }

    pub fn cell_at(&self, world_position: IVec3) -> Option<VoxelCell> {
        if world_position.y < 0 {
            return None;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);

        self.chunks.get(&chunk_coord)?.cell_at(
            local_position.x,
            local_position.y,
            local_position.z,
        )
    }

    pub fn fluid_at(&self, world_position: IVec3) -> Option<FluidCell> {
        if world_position.y < 0 {
            return None;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);

        self.chunks.get(&chunk_coord)?.fluid_at(
            local_position.x,
            local_position.y,
            local_position.z,
        )
    }

    pub(crate) fn skylight_at(&self, world_position: IVec3) -> u8 {
        if world_position.y < 0 {
            return 0;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);
        let Some(chunk) = self.chunks.get(&chunk_coord) else {
            return 0;
        };

        chunk.skylight_at(local_position.x, local_position.y, local_position.z)
    }

    pub fn is_loaded_at(&self, world_position: IVec3) -> bool {
        if world_position.y < 0 {
            return false;
        }

        let (chunk_coord, _) = split_world_position(world_position);
        self.chunks.contains_key(&chunk_coord)
    }

    pub(crate) fn highest_loaded_chunk_y(&self) -> i32 {
        self.chunks.keys().map(|coord| coord.y).max().unwrap_or(0)
    }

    pub(crate) fn highest_solid_y_in_column(
        &self,
        world_x: i32,
        world_z: i32,
        max_chunk_y: i32,
    ) -> Option<i32> {
        let chunk_size = CHUNK_SIZE as i32;
        let chunk_x = world_x.div_euclid(chunk_size);
        let chunk_z = world_z.div_euclid(chunk_size);
        let local_x = world_x.rem_euclid(chunk_size);
        let local_z = world_z.rem_euclid(chunk_size);

        for chunk_y in (0..=max_chunk_y).rev() {
            let Some(chunk) = self.chunks.get(&IVec3::new(chunk_x, chunk_y, chunk_z)) else {
                continue;
            };
            let Some(local_y) = chunk.highest_solid_y(local_x, local_z) else {
                continue;
            };

            return Some(chunk_y * chunk_size + local_y);
        }

        None
    }

    pub fn set_block_at(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<IVec3> {
        if world_position.y < 0 {
            return None;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);

        {
            let chunk = self.chunks.get_mut(&chunk_coord)?;
            let x = local_position.x as usize;
            let y = local_position.y as usize;
            let z = local_position.z as usize;

            chunk.set_block(x, y, z, block);

            if block.is_some() {
                chunk.set_fluid(x, y, z, None);
            }
        }

        relight_chunk_and_neighbors(self, chunk_coord);
        Some(chunk_coord)
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
