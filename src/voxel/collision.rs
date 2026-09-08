use bevy::prelude::*;

use super::world::VoxelWorld;

const COLLISION_EPSILON: f32 = 0.0001;

pub fn collides_aabb(world: &VoxelWorld, min: Vec3, max: Vec3) -> bool {
    let min = min + Vec3::splat(COLLISION_EPSILON);
    let max = max - Vec3::splat(COLLISION_EPSILON);

    let min = min.floor().as_ivec3();
    let max = max.floor().as_ivec3();

    for y in min.y..=max.y {
        for z in min.z..=max.z {
            for x in min.x..=max.x {
                if world.is_solid(IVec3::new(x, y, z)) {
                    return true;
                }
            }
        }
    }

    false
}
