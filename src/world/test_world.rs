use bevy::prelude::*;

use crate::voxel::{
    chunk::{VoxelChunk, CHUNK_SIZE},
    world::VoxelWorld,
};

use super::render_distance::chunk_coords_in_radius;

pub fn build_test_world(center: IVec2, radius: i32) -> VoxelWorld {
    let mut world = VoxelWorld::default();

    for coord in chunk_coords_in_radius(center, radius) {
        let chunk = if coord == center {
            central_test_chunk()
        } else {
            varied_terrain_chunk(coord)
        };

        world.insert_chunk(coord, chunk);
    }

    world
}

fn central_test_chunk() -> VoxelChunk {
    let mut chunk = VoxelChunk::empty();

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set_solid(x, 0, z, true);
        }
    }

    for z in 3..13 {
        for y in 1..4 {
            chunk.set_solid(4, y, z, true);
        }
    }

    for x in 8..12 {
        for z in 4..7 {
            chunk.set_solid(x, 3, z, true);
        }
    }

    for x in 10..13 {
        for z in 10..13 {
            chunk.set_solid(x, 1, z, true);
        }
    }

    chunk
}

fn varied_terrain_chunk(coord: IVec2) -> VoxelChunk {
    let mut chunk = VoxelChunk::empty();
    let chunk_size = CHUNK_SIZE as i32;

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = coord.x * chunk_size + local_x as i32;
            let world_z = coord.y * chunk_size + local_z as i32;
            let height = terrain_height(world_x, world_z);

            for y in 0..height {
                chunk.set_solid(local_x, y, local_z, true);
            }
        }
    }

    chunk
}

fn terrain_height(world_x: i32, world_z: i32) -> usize {
    let x = world_x as f32;
    let z = world_z as f32;

    let broad_hills = (x * 0.035).sin() * 2.2 + (z * 0.04).cos() * 1.8;
    let crossing_ridge = ((x + z) * 0.022).sin() * 1.6;
    let small_variation = ((x - z) * 0.075).cos() * 0.8;

    (4.0 + broad_hills + crossing_ridge + small_variation)
        .round()
        .clamp(1.0, 10.0) as usize
}
