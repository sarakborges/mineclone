use bevy::prelude::*;

use super::{
    cell::VoxelCell,
    log_variant::is_hollow_log_id,
    microblock::{HOLLOW_LOG_EDGE, MICROBLOCK_EDGE, MicroblockMask},
    world::VoxelWorld,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoxelHit {
    pub voxel: IVec3,
    pub block_id: &'static str,
    pub normal: IVec3,
}

/// Exact occupied cell in the fixed 8x8x8 precision grid. A macro block with
/// no Artisan's Kit mask behaves as 512 occupied subcells without extra allocation.
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
    let direction = direction.normalize() * HOLLOW_LOG_EDGE as f32;
    let origin = origin * HOLLOW_LOG_EDGE as f32;
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
        if let Some((cell, voxel, artisan_fine)) = occupied_raycast_cell(world, fine) {
            return Some(MicroVoxelHit {
                voxel,
                fine: artisan_fine,
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

fn occupied_raycast_cell(
    world: &VoxelWorld,
    fine: IVec3,
) -> Option<(VoxelCell, IVec3, IVec3)> {
    let voxel = IVec3::new(
        fine.x.div_euclid(HOLLOW_LOG_EDGE),
        fine.y.div_euclid(HOLLOW_LOG_EDGE),
        fine.z.div_euclid(HOLLOW_LOG_EDGE),
    );
    let local = [
        fine.x.rem_euclid(HOLLOW_LOG_EDGE) as usize,
        fine.y.rem_euclid(HOLLOW_LOG_EDGE) as usize,
        fine.z.rem_euclid(HOLLOW_LOG_EDGE) as usize,
    ];
    let cell = world.cell_at(voxel)?;

    let occupied = if MicroblockMask::is_modified(cell) {
        let artisan_local = local.map(|axis| axis / 2);
        MicroblockMask::from_cell(cell).contains(artisan_local)
    } else if is_hollow_log_id(cell.block_id) {
        MicroblockMask::hollow_log_contains(cell.orientation, local)
    } else {
        true
    };
    if !occupied {
        return None;
    }

    let artisan_local = IVec3::new(
        (local[0] / 2) as i32,
        (local[1] / 2) as i32,
        (local[2] / 2) as i32,
    );
    let artisan_fine = voxel * MICROBLOCK_EDGE + artisan_local;
    Some((cell, voxel, artisan_fine))
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
