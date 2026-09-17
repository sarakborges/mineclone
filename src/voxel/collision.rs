use bevy::prelude::*;

use super::{microblock::MicroblockMask, world::VoxelWorld};

const COLLISION_EPSILON: f32 = 0.0001;
const MICRO_EDGE: f32 = 8.0;

pub fn collides_aabb(world: &VoxelWorld, min: Vec3, max: Vec3) -> bool {
    let min = min + Vec3::splat(COLLISION_EPSILON);
    let max = max - Vec3::splat(COLLISION_EPSILON);

    let min_voxel = min.floor().as_ivec3();
    let max_voxel = max.floor().as_ivec3();

    for y in min_voxel.y..=max_voxel.y {
        for z in min_voxel.z..=max_voxel.z {
            for x in min_voxel.x..=max_voxel.x {
                let voxel = IVec3::new(x, y, z);
                let Some(cell) = world.cell_at(voxel) else {
                    continue;
                };
                if !MicroblockMask::is_modified(cell) {
                    return true;
                }

                let shape = MicroblockMask::from_cell(cell);
                let origin = voxel.as_vec3();
                let min_cell = ((min - origin) * MICRO_EDGE)
                    .floor()
                    .as_ivec3()
                    .clamp(IVec3::ZERO, IVec3::splat(7));
                let max_cell = ((max - origin) * MICRO_EDGE)
                    .floor()
                    .as_ivec3()
                    .clamp(IVec3::ZERO, IVec3::splat(7));
                for cz in min_cell.z..=max_cell.z {
                    for cy in min_cell.y..=max_cell.y {
                        for cx in min_cell.x..=max_cell.x {
                            if shape.contains([cx as usize, cy as usize, cz as usize]) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    false
}
