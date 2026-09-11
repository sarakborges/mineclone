use bevy::prelude::*;

use crate::voxel::{
    chunk::CHUNK_SIZE, neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
};

use super::PendingFluidUpdates;

const FLUID_SPREAD_TARGETS: [IVec3; 5] = [
    IVec3::NEG_Y,
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Z,
    IVec3::NEG_Z,
];

pub(super) fn enqueue_loaded_fluid_frontier(
    pending: &mut PendingFluidUpdates,
    world: &VoxelWorld,
    coord: IVec3,
) {
    enqueue_chunk_spread_targets(pending, world, coord);

    for offset in CARDINAL_NEIGHBORS {
        enqueue_neighbor_boundary_spread_targets(pending, world, coord + offset, -offset);
    }
}

fn enqueue_chunk_spread_targets(
    pending: &mut PendingFluidUpdates,
    world: &VoxelWorld,
    coord: IVec3,
) {
    if coord.y < 0 {
        return;
    }
    let Some(chunk) = world.chunk(coord) else {
        return;
    };

    let origin = coord * CHUNK_SIZE as i32;

    for local_y in 0..CHUNK_SIZE {
        for local_z in 0..CHUNK_SIZE {
            for local_x in 0..CHUNK_SIZE {
                if chunk
                    .fluid_at(local_x as i32, local_y as i32, local_z as i32)
                    .is_none()
                {
                    continue;
                }

                let position =
                    origin + IVec3::new(local_x as i32, local_y as i32, local_z as i32);
                enqueue_spread_targets_from_fluid(pending, world, position);
            }
        }
    }
}

fn enqueue_neighbor_boundary_spread_targets(
    pending: &mut PendingFluidUpdates,
    world: &VoxelWorld,
    coord: IVec3,
    direction: IVec3,
) {
    if coord.y < 0 {
        return;
    }
    let Some(chunk) = world.chunk(coord) else {
        return;
    };

    let size = CHUNK_SIZE as i32;
    let origin = coord * size;

    if direction.x != 0 {
        let local_x = if direction.x < 0 { 0 } else { size - 1 };
        for local_y in 0..size {
            for local_z in 0..size {
                if chunk.fluid_at(local_x, local_y, local_z).is_none() {
                    continue;
                }
                enqueue_spread_targets_from_fluid(
                    pending,
                    world,
                    origin + IVec3::new(local_x, local_y, local_z),
                );
            }
        }
        return;
    }

    if direction.y != 0 {
        let local_y = if direction.y < 0 { 0 } else { size - 1 };
        for local_z in 0..size {
            for local_x in 0..size {
                if chunk.fluid_at(local_x, local_y, local_z).is_none() {
                    continue;
                }
                enqueue_spread_targets_from_fluid(
                    pending,
                    world,
                    origin + IVec3::new(local_x, local_y, local_z),
                );
            }
        }
        return;
    }

    let local_z = if direction.z < 0 { 0 } else { size - 1 };
    for local_y in 0..size {
        for local_x in 0..size {
            if chunk.fluid_at(local_x, local_y, local_z).is_none() {
                continue;
            }
            enqueue_spread_targets_from_fluid(
                pending,
                world,
                origin + IVec3::new(local_x, local_y, local_z),
            );
        }
    }
}

fn enqueue_spread_targets_from_fluid(
    pending: &mut PendingFluidUpdates,
    world: &VoxelWorld,
    position: IVec3,
) {
    for offset in FLUID_SPREAD_TARGETS {
        let target = position + offset;
        if !world.is_loaded_at(target) || world.is_solid(target) || world.fluid_at(target).is_some() {
            continue;
        }

        pending.enqueue(target);
    }
}
