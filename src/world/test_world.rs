use bevy::prelude::*;

use crate::{
    content::block::BlockRegistry,
    voxel::{
        cell::VoxelCell,
        chunk::{VoxelChunk, CHUNK_SIZE},
        texture_rotation::TextureRotation,
        world::VoxelWorld,
    },
};

use super::render_distance::chunk_coords_in_radius;

const GRASS_BLOCK_ID: &str = "mineclone:grass";

pub fn build_test_world(center: IVec2, radius: i32, blocks: &BlockRegistry) -> VoxelWorld {
    let grass = blocks
        .get(GRASS_BLOCK_ID)
        .unwrap_or_else(|| panic!("missing block definition: {GRASS_BLOCK_ID}"));
    let mut world = VoxelWorld::default();

    for coord in chunk_coords_in_radius(center, radius) {
        world.insert_chunk(coord, varied_terrain_chunk(coord, grass.rotate_texture));
    }

    world
}

fn varied_terrain_chunk(coord: IVec2, rotate_texture: bool) -> VoxelChunk {
    let mut chunk = VoxelChunk::empty();
    let chunk_size = CHUNK_SIZE as i32;

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = coord.x * chunk_size + local_x as i32;
            let world_z = coord.y * chunk_size + local_z as i32;
            let height = terrain_height(world_x, world_z);

            for y in 0..height {
                let world_position = IVec3::new(world_x, y as i32, world_z);
                let rotation = texture_rotation_for(world_position, rotate_texture);
                chunk.set_block(
                    local_x,
                    y,
                    local_z,
                    Some(VoxelCell::new(GRASS_BLOCK_ID, rotation)),
                );
            }
        }
    }

    chunk
}

fn texture_rotation_for(position: IVec3, enabled: bool) -> TextureRotation {
    if !enabled {
        return TextureRotation::default();
    }

    let mut hash = position.x as u32;
    hash ^= (position.y as u32).wrapping_mul(0x9e37_79b9);
    hash = hash.rotate_left(13);
    hash ^= (position.z as u32).wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 16;

    TextureRotation::from_quarter_turn((hash & 3) as u8)
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
