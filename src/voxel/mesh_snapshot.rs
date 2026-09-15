use bevy::prelude::*;

use super::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, VoxelChunk},
    coordinates::chunk_origin,
    fluid::FluidCell,
    light::VoxelLight,
    read::VoxelRead,
    world::VoxelWorld,
};

const HALO: i32 = 1;
const SNAPSHOT_SIDE: usize = CHUNK_SIZE + 2;
const SNAPSHOT_FACE_AREA: usize = SNAPSHOT_SIDE * SNAPSHOT_SIDE;
const INTERIOR_SHELL_LAYER: usize = SNAPSHOT_SIDE * 2 + CHUNK_SIZE * 2;
const SHELL_VOLUME: usize = SNAPSHOT_FACE_AREA * 2 + INTERIOR_SHELL_LAYER * CHUNK_SIZE;

pub(crate) struct ChunkMeshSnapshot {
    chunk_origin: IVec3,
    chunk: VoxelChunk,
    shell_cells: Box<[Option<VoxelCell>]>,
    shell_fluids: Box<[Option<FluidCell>]>,
    shell_light: Box<[VoxelLight]>,
    shell_loaded: Box<[bool]>,
}

impl ChunkMeshSnapshot {
    pub(crate) fn capture(world: &VoxelWorld, coord: IVec3) -> Option<Self> {
        let chunk = world.chunk(coord)?.clone();
        let chunk_origin = chunk_origin(coord);
        let snapshot_origin = chunk_origin - IVec3::splat(HALO);
        let mut shell_cells = vec![None; SHELL_VOLUME].into_boxed_slice();
        let mut shell_fluids = vec![None; SHELL_VOLUME].into_boxed_slice();
        let mut shell_light = vec![VoxelLight::DARK; SHELL_VOLUME].into_boxed_slice();
        let mut shell_loaded = vec![false; SHELL_VOLUME].into_boxed_slice();
        let last = SNAPSHOT_SIDE - 1;
        let mut capture_shell_voxel = |x: usize, y: usize, z: usize| {
            let index = shell_index_from_snapshot_coords(x, y, z)
                .expect("mesh snapshot shell coordinates must have a compact index");
            let position = snapshot_origin + IVec3::new(x as i32, y as i32, z as i32);
            let Some((cell, fluid, light)) = world.sample_at(position) else {
                return;
            };

            shell_loaded[index] = true;
            shell_cells[index] = cell;
            shell_fluids[index] = fluid;
            shell_light[index] = light;
        };

        for z in 0..SNAPSHOT_SIDE {
            for x in 0..SNAPSHOT_SIDE {
                capture_shell_voxel(x, 0, z);
                capture_shell_voxel(x, last, z);
            }
        }

        for y in 1..last {
            for x in 0..SNAPSHOT_SIDE {
                capture_shell_voxel(x, y, 0);
                capture_shell_voxel(x, y, last);
            }
            for z in 1..last {
                capture_shell_voxel(0, y, z);
                capture_shell_voxel(last, y, z);
            }
        }

        Some(Self {
            chunk_origin,
            chunk,
            shell_cells,
            shell_fluids,
            shell_light,
            shell_loaded,
        })
    }

    pub(crate) fn chunk(&self) -> &VoxelChunk {
        &self.chunk
    }

    fn central_local(&self, position: IVec3) -> Option<IVec3> {
        let local = position - self.chunk_origin;
        inside_chunk(local).then_some(local)
    }

    fn shell_index(&self, position: IVec3) -> Option<usize> {
        let snapshot_origin = self.chunk_origin - IVec3::splat(HALO);
        let local = position - snapshot_origin;
        if local.x < 0
            || local.y < 0
            || local.z < 0
            || local.x >= SNAPSHOT_SIDE as i32
            || local.y >= SNAPSHOT_SIDE as i32
            || local.z >= SNAPSHOT_SIDE as i32
        {
            return None;
        }

        shell_index_from_snapshot_coords(local.x as usize, local.y as usize, local.z as usize)
    }
}

impl VoxelRead for ChunkMeshSnapshot {
    fn cell_at(&self, world_position: IVec3) -> Option<VoxelCell> {
        if let Some(local) = self.central_local(world_position) {
            return self.chunk.cell_at(local.x, local.y, local.z);
        }
        self.shell_index(world_position)
            .and_then(|index| self.shell_loaded[index].then_some(self.shell_cells[index]))
            .flatten()
    }

    fn fluid_at(&self, world_position: IVec3) -> Option<FluidCell> {
        if let Some(local) = self.central_local(world_position) {
            return self.chunk.fluid_at(local.x, local.y, local.z);
        }
        self.shell_index(world_position)
            .and_then(|index| self.shell_loaded[index].then_some(self.shell_fluids[index]))
            .flatten()
    }

    fn light_at(&self, world_position: IVec3) -> VoxelLight {
        if let Some(local) = self.central_local(world_position) {
            return self.chunk.light_at(local.x, local.y, local.z);
        }
        self.shell_index(world_position)
            .filter(|index| self.shell_loaded[*index])
            .map_or(VoxelLight::DARK, |index| self.shell_light[index])
    }

    fn is_loaded_at(&self, world_position: IVec3) -> bool {
        if self.central_local(world_position).is_some() {
            return true;
        }
        self.shell_index(world_position)
            .is_some_and(|index| self.shell_loaded[index])
    }
}

fn inside_chunk(local: IVec3) -> bool {
    local.x >= 0
        && local.y >= 0
        && local.z >= 0
        && local.x < CHUNK_SIZE as i32
        && local.y < CHUNK_SIZE as i32
        && local.z < CHUNK_SIZE as i32
}

fn shell_index_from_snapshot_coords(x: usize, y: usize, z: usize) -> Option<usize> {
    let last = SNAPSHOT_SIDE - 1;

    if y == 0 {
        return Some(x + z * SNAPSHOT_SIDE);
    }
    if y == last {
        return Some(SNAPSHOT_FACE_AREA + x + z * SNAPSHOT_SIDE);
    }

    let layer = SNAPSHOT_FACE_AREA * 2 + (y - 1) * INTERIOR_SHELL_LAYER;
    if z == 0 {
        return Some(layer + x);
    }
    if z == last {
        return Some(layer + SNAPSHOT_SIDE + x);
    }
    if x == 0 {
        return Some(layer + SNAPSHOT_SIDE * 2 + (z - 1) * 2);
    }
    if x == last {
        return Some(layer + SNAPSHOT_SIDE * 2 + (z - 1) * 2 + 1);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{
        cell::VoxelCell, texture_rotation::TextureRotation, world::VoxelWorld,
    };

    #[test]
    fn snapshot_preserves_central_chunk_and_one_voxel_halo() {
        let mut world = VoxelWorld::default();
        let coord = IVec3::ZERO;
        let mut center = VoxelChunk::empty();
        center.set_block(
            0,
            0,
            0,
            Some(VoxelCell::new("asteria:test", TextureRotation::default())),
        );
        world.insert_chunk(coord, center);

        let mut neighbor = VoxelChunk::empty();
        neighbor.set_block(
            0,
            0,
            0,
            Some(VoxelCell::new(
                "asteria:neighbor",
                TextureRotation::default(),
            )),
        );
        world.insert_chunk(IVec3::X, neighbor);

        let snapshot = ChunkMeshSnapshot::capture(&world, coord).expect("chunk should exist");
        assert_eq!(snapshot.block_id_at(IVec3::ZERO), Some("asteria:test"));
        assert_eq!(
            snapshot.block_id_at(IVec3::new(CHUNK_SIZE as i32, 0, 0)),
            Some("asteria:neighbor")
        );
        assert!(!snapshot.is_loaded_at(IVec3::new(CHUNK_SIZE as i32 + 1, 0, 0)));
    }

    #[test]
    fn compact_shell_index_covers_each_shell_voxel_once() {
        let mut seen = vec![false; SHELL_VOLUME];
        let mut count = 0;

        for y in 0..SNAPSHOT_SIDE {
            for z in 0..SNAPSHOT_SIDE {
                for x in 0..SNAPSHOT_SIDE {
                    let Some(index) = shell_index_from_snapshot_coords(x, y, z) else {
                        continue;
                    };
                    assert!(index < SHELL_VOLUME);
                    assert!(!seen[index]);
                    seen[index] = true;
                    count += 1;
                }
            }
        }

        assert_eq!(count, SHELL_VOLUME);
        assert!(seen.into_iter().all(|value| value));
    }
}
