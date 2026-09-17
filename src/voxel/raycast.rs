use bevy::prelude::*;

use super::{
    microblock::{MICROBLOCK_EDGE, occupied_cell, parent_voxel},
    world::VoxelWorld,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoxelHit {
    pub voxel: IVec3,
    pub block_id: &'static str,
    pub normal: IVec3,
}

/// Exact occupied cell in the fixed 8x8x8 precision grid. A macro block with
/// no Chisel mask behaves as 512 occupied subcells without extra allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MicroVoxelHit {
    pub(crate) voxel: IVec3,
    pub(crate) fine: IVec3,
    pub(crate) block_id: &'static str,
    pub(crate) normal: IVec3,
}

pub fn raycast_voxels(
    world: &VoxelWorld,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<VoxelHit> {
    raycast_micro_voxels(world, origin, direction, max_distance).map(|hit| VoxelHit {
        voxel: hit.voxel,
        block_id: hit.block_id,
        normal: hit.normal,
    })
}

pub(crate) fn raycast_micro_voxels(
    world: &VoxelWorld,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<MicroVoxelHit> {
    if direction.length_squared() == 0.0 || max_distance < 0.0 {
        return None;
    }

    // Scale both origin and velocity. DDA times remain measured in macro-block
    // world units, so the existing target-range semantics are unchanged.
    let direction = direction.normalize() * MICROBLOCK_EDGE as f32;
    let origin = origin * MICROBLOCK_EDGE as f32;
    let mut fine = origin.floor().as_ivec3();
    let step = IVec3::new(
        direction.x.signum() as i32,
        direction.y.signum() as i32,
        direction.z.signum() as i32,
    );
    let mut entry_normal = IVec3::ZERO;

    let t_delta = Vec3::new(
        reciprocal_abs(direction.x),
        reciprocal_abs(direction.y),
        reciprocal_abs(direction.z),
    );
    let mut t_max = Vec3::new(
        first_boundary_distance(origin.x, fine.x, direction.x),
        first_boundary_distance(origin.y, fine.y, direction.y),
        first_boundary_distance(origin.z, fine.z, direction.z),
    );

    loop {
        if let Some(cell) = occupied_cell(world, fine) {
            return Some(MicroVoxelHit {
                voxel: parent_voxel(fine),
                fine,
                block_id: cell.block_id,
                normal: entry_normal,
            });
        }

        let distance = if t_max.x <= t_max.y && t_max.x <= t_max.z {
            let distance = t_max.x;
            fine.x += step.x;
            entry_normal = IVec3::new(-step.x, 0, 0);
            t_max.x += t_delta.x;
            distance
        } else if t_max.y <= t_max.z {
            let distance = t_max.y;
            fine.y += step.y;
            entry_normal = IVec3::new(0, -step.y, 0);
            t_max.y += t_delta.y;
            distance
        } else {
            let distance = t_max.z;
            fine.z += step.z;
            entry_normal = IVec3::new(0, 0, -step.z);
            t_max.z += t_delta.z;
            distance
        };

        if distance > max_distance {
            return None;
        }
    }
}

fn reciprocal_abs(value: f32) -> f32 {
    if value == 0.0 {
        f32::INFINITY
    } else {
        1.0 / value.abs()
    }
}

fn first_boundary_distance(origin: f32, voxel: i32, direction: f32) -> f32 {
    if direction > 0.0 {
        (voxel as f32 + 1.0 - origin) / direction
    } else if direction < 0.0 {
        (origin - voxel as f32) / -direction
    } else {
        f32::INFINITY
    }
}
