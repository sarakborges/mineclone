use bevy::prelude::*;

use super::{microblock::MicroblockMask, world::VoxelWorld};

const COLLISION_EPSILON: f32 = 0.0001;
const MICRO_EDGE: f32 = 8.0;
pub const ENTITY_STEP_HEIGHT: f32 = 0.5;
pub const ENTITY_STEP_INCREMENT: f32 = 0.125;

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

/// Try to climb a collision that is no taller than the entity step height.
/// Geometry is evaluated at the world's 1/8-block micro resolution so a
/// quarter-block or smaller ledge is climbed by exactly the required amount.
pub fn try_step_up_aabb(
    world: &VoxelWorld,
    position: Vec3,
    horizontal_delta: Vec3,
    max_step_height: f32,
    bounds_at: impl Fn(Vec3) -> (Vec3, Vec3),
) -> Option<Vec3> {
    if horizontal_delta.x == 0.0 && horizontal_delta.z == 0.0 {
        return None;
    }

    let max_step_height = max_step_height.max(0.0);
    if max_step_height < ENTITY_STEP_INCREMENT {
        return None;
    }

    let mut rise = ENTITY_STEP_INCREMENT;
    while rise <= max_step_height + COLLISION_EPSILON {
        let elevated = position + Vec3::Y * rise;
        if aabb_is_clear(world, bounds_at(elevated))
            && let Some(horizontal_position) =
                move_aabb_horizontally(world, elevated, horizontal_delta, &bounds_at)
            && let Some(settled) =
                settle_after_step(world, horizontal_position, rise, &bounds_at)
        {
            return Some(settled);
        }
        rise += ENTITY_STEP_INCREMENT;
    }

    None
}

fn aabb_is_clear(world: &VoxelWorld, bounds: (Vec3, Vec3)) -> bool {
    let min = (bounds.0 + Vec3::splat(COLLISION_EPSILON)).floor().as_ivec3();
    let max = (bounds.1 - Vec3::splat(COLLISION_EPSILON)).floor().as_ivec3();
    for y in min.y..=max.y {
        for z in min.z..=max.z {
            for x in min.x..=max.x {
                if !world.is_loaded_at(IVec3::new(x, y, z)) {
                    return false;
                }
            }
        }
    }
    !collides_aabb(world, bounds.0, bounds.1)
}

fn move_aabb_horizontally(
    world: &VoxelWorld,
    position: Vec3,
    horizontal_delta: Vec3,
    bounds_at: &impl Fn(Vec3) -> (Vec3, Vec3),
) -> Option<Vec3> {
    let steps = (horizontal_delta.length() / 0.05).ceil().max(1.0) as usize;
    let step = horizontal_delta / steps as f32;
    let mut current = position;
    for _ in 0..steps {
        let next = current + step;
        if !aabb_is_clear(world, bounds_at(next)) {
            return None;
        }
        current = next;
    }
    Some(current)
}

fn settle_after_step(
    world: &VoxelWorld,
    position: Vec3,
    rise: f32,
    bounds_at: &impl Fn(Vec3) -> (Vec3, Vec3),
) -> Option<Vec3> {
    let steps = (rise / ENTITY_STEP_INCREMENT).ceil().max(1.0) as usize;
    let step = rise / steps as f32;
    let mut current = position;
    for _ in 0..steps {
        let next = current - Vec3::Y * step;
        if !aabb_is_clear(world, bounds_at(next)) {
            return Some(current);
        }
        current = next;
    }
    None
}
