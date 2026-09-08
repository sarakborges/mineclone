use bevy::prelude::*;

use crate::voxel_chunk::{VoxelChunk, CHUNK_SIZE};

const COLLISION_EPSILON: f32 = 0.0001;

pub fn collides_aabb(chunk: &VoxelChunk, min: Vec3, max: Vec3) -> bool {
    let chunk_size = CHUNK_SIZE as f32;

    if max.x <= 0.0
        || max.y <= 0.0
        || max.z <= 0.0
        || min.x >= chunk_size
        || min.y >= chunk_size
        || min.z >= chunk_size
    {
        return false;
    }

    let min_x = min.x.floor().max(0.0) as i32;
    let min_y = min.y.floor().max(0.0) as i32;
    let min_z = min.z.floor().max(0.0) as i32;
    let max_x = (max.x - COLLISION_EPSILON)
        .floor()
        .min((CHUNK_SIZE - 1) as f32) as i32;
    let max_y = (max.y - COLLISION_EPSILON)
        .floor()
        .min((CHUNK_SIZE - 1) as f32) as i32;
    let max_z = (max.z - COLLISION_EPSILON)
        .floor()
        .min((CHUNK_SIZE - 1) as f32) as i32;

    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                if chunk.is_solid(x, y, z) {
                    return true;
                }
            }
        }
    }

    false
}
