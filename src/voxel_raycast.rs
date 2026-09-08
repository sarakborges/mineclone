use bevy::prelude::*;

use crate::voxel_chunk::VoxelChunk;

#[derive(Clone, Copy)]
pub struct VoxelHit {
    pub voxel: IVec3,
    pub block_id: &'static str,
}

pub fn raycast_voxels(
    chunk: &VoxelChunk,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<VoxelHit> {
    if direction.length_squared() == 0.0 {
        return None;
    }

    let direction = direction.normalize();
    let mut voxel = origin.floor().as_ivec3();
    let step = IVec3::new(
        direction.x.signum() as i32,
        direction.y.signum() as i32,
        direction.z.signum() as i32,
    );

    let t_delta = Vec3::new(
        reciprocal_abs(direction.x),
        reciprocal_abs(direction.y),
        reciprocal_abs(direction.z),
    );
    let mut t_max = Vec3::new(
        first_boundary_distance(origin.x, voxel.x, direction.x),
        first_boundary_distance(origin.y, voxel.y, direction.y),
        first_boundary_distance(origin.z, voxel.z, direction.z),
    );

    loop {
        if let Some(block_id) = chunk.block_id_at(voxel.x, voxel.y, voxel.z) {
            return Some(VoxelHit { voxel, block_id });
        }

        let distance = if t_max.x <= t_max.y && t_max.x <= t_max.z {
            let distance = t_max.x;
            voxel.x += step.x;
            t_max.x += t_delta.x;
            distance
        } else if t_max.y <= t_max.z {
            let distance = t_max.y;
            voxel.y += step.y;
            t_max.y += t_delta.y;
            distance
        } else {
            let distance = t_max.z;
            voxel.z += step.z;
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
