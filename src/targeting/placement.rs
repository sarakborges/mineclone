use bevy::prelude::*;

use crate::{
    player::{PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT},
    voxel::{raycast::VoxelHit, world::VoxelWorld},
};

pub(crate) fn placement_voxel(
    hit: VoxelHit,
    world: &VoxelWorld,
    player_eye_position: Vec3,
) -> Option<IVec3> {
    if hit.normal == IVec3::ZERO {
        return None;
    }

    let voxel = hit.voxel + hit.normal;

    if voxel.y < 0
        || !world.is_loaded_at(voxel)
        || world.is_solid(voxel)
        || block_intersects_player(voxel, player_eye_position)
    {
        return None;
    }

    Some(voxel)
}

fn block_intersects_player(voxel: IVec3, eye_position: Vec3) -> bool {
    let feet_y = eye_position.y - PLAYER_EYE_HEIGHT;
    let player_min = Vec3::new(
        eye_position.x - PLAYER_HALF_WIDTH,
        feet_y,
        eye_position.z - PLAYER_HALF_WIDTH,
    );
    let player_max = Vec3::new(
        eye_position.x + PLAYER_HALF_WIDTH,
        feet_y + PLAYER_HEIGHT,
        eye_position.z + PLAYER_HALF_WIDTH,
    );
    let block_min = voxel.as_vec3();
    let block_max = block_min + Vec3::ONE;

    player_min.x < block_max.x
        && player_max.x > block_min.x
        && player_min.y < block_max.y
        && player_max.y > block_min.y
        && player_min.z < block_max.z
        && player_max.z > block_min.z
}
