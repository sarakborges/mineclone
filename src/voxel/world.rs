use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use super::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, VoxelChunk},
    chunk_archive::ArchivedChunk,
    fluid::FluidCell,
    light::VoxelLight,
};

#[derive(Resource, Default)]
pub struct VoxelWorld {
    chunks: HashMap<IVec3, VoxelChunk>,
    archived_chunks: HashMap<IVec3, ArchivedChunk>,
    generated_chunks: HashSet<IVec3>,
    dirty_chunks: HashSet<IVec3>,
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
    }

    pub fn chunk(&self, coord: IVec3) -> Option<&VoxelChunk> {
        if coord.y < 0 {
            return None;
        }

        self.chunks.get(&coord)
    }

    pub fn archive_chunk(&mut self, coord: IVec3) {
        let Some(chunk) = self.chunks.remove(&coord) else {
            return;
        };

        if self.dirty_chunks.contains(&coord) {
            self.archived_chunks
                .insert(coord, ArchivedChunk::from_chunk(&chunk));
        } else {
            self.generated_chunks.remove(&coord);
        }
    }

    pub fn restore_chunk(&mut self, coord: IVec3) -> bool {
        if self.chunks.contains_key(&coord) {
            return true;
        }

        let Some(archived) = self.archived_chunks.remove(&coord) else {
            return false;
        };

        self.chunks.insert(coord, archived.restore());
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

        self.chunks
            .get(&chunk_coord)?
            .cell_at(local_position.x, local_position.y, local_position.z)
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

    pub(crate) fn light_at(&self, world_position: IVec3) -> VoxelLight {
        if world_position.y < 0 {
            return VoxelLight::DARK;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);
        let Some(chunk) = self.chunks.get(&chunk_coord) else {
            return VoxelLight::DARK;
        };

        chunk.light_at(local_position.x, local_position.y, local_position.z)
    }

    pub(crate) fn set_light_at(&mut self, world_position: IVec3, light: VoxelLight) -> bool {
        if world_position.y < 0 {
            return false;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);
        let Some(chunk) = self.chunks.get_mut(&chunk_coord) else {
            return false;
        };

        chunk.set_light(
            local_position.x as usize,
            local_position.y as usize,
            local_position.z as usize,
            light,
        )
    }

    pub(crate) fn clear_chunk_light(&mut self, coord: IVec3) -> bool {
        let Some(chunk) = self.chunks.get_mut(&coord) else {
            return false;
        };

        chunk.clear_light();
        true
    }

    pub fn is_loaded_at(&self, world_position: IVec3) -> bool {
        if world_position.y < 0 {
            return false;
        }

        let (chunk_coord, _) = split_world_position(world_position);
        self.chunks.contains_key(&chunk_coord)
    }

    pub(crate) fn highest_loaded_world_y_in_column(
        &self,
        world_x: i32,
        world_z: i32,
    ) -> Option<i32> {
        let chunk_size = CHUNK_SIZE as i32;
        let chunk_x = world_x.div_euclid(chunk_size);
        let chunk_z = world_z.div_euclid(chunk_size);

        self.chunks
            .keys()
            .filter(|coord| coord.x == chunk_x && coord.z == chunk_z)
            .map(|coord| (coord.y + 1) * chunk_size - 1)
            .max()
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

        self.dirty_chunks.insert(chunk_coord);
        Some(chunk_coord)
    }

    pub(crate) fn set_fluid_at(
        &mut self,
        world_position: IVec3,
        fluid: Option<FluidCell>,
    ) -> Option<IVec3> {
        if world_position.y < 0 {
            return None;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);
        let chunk = self.chunks.get_mut(&chunk_coord)?;
        let x = local_position.x as usize;
        let y = local_position.y as usize;
        let z = local_position.z as usize;

        if fluid.is_some() && chunk.cell_at(local_position.x, local_position.y, local_position.z).is_some() {
            return None;
        }
        if chunk.fluid_at(local_position.x, local_position.y, local_position.z) == fluid {
            return None;
        }

        chunk.set_fluid(x, y, z, fluid);
        self.dirty_chunks.insert(chunk_coord);
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
