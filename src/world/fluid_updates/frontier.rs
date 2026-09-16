use bevy::prelude::*;

use crate::voxel::{
    chunk::{CHUNK_SIZE, VoxelChunk},
    neighbors::CARDINAL_NEIGHBORS,
    world::VoxelWorld,
};

use super::PendingFluidUpdates;

const FLUID_SPREAD_TARGETS: [IVec3; 5] =
    [IVec3::NEG_Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

pub(super) fn enqueue_loaded_fluid_frontier(
    pending: &mut PendingFluidUpdates,
    world: &VoxelWorld,
    coord: IVec3,
) {
    // Natural hydrology is already rasterized at its final water volume during
    // chunk generation. Re-scanning every generated fluid voxel here used to
    // turn that finished water back into simulation work on every chunk load.
    // Only resume fluid that was already dynamically spreading across a loaded
    // neighbor boundary.
    for offset in CARDINAL_NEIGHBORS {
        enqueue_neighbor_boundary_spread_targets(pending, world, coord + offset, -offset);
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
    let mut remaining_dynamic = chunk.boundary_dynamic_fluid_count(direction);
    if remaining_dynamic == 0 {
        return;
    }

    // Every dynamic boundary fluid can contribute at most the five solver
    // spread targets below. Reserve once from metadata before scanning the face
    // so a resumed frontier does not repeatedly grow the deduplicated queue.
    pending.reserve(remaining_dynamic.saturating_mul(FLUID_SPREAD_TARGETS.len()));

    let size = CHUNK_SIZE as i32;
    let origin = coord * size;

    if direction.x != 0 {
        let local_x = if direction.x < 0 { 0 } else { size - 1 };
        for local_y in 0..size {
            for local_z in 0..size {
                if !is_dynamic_fluid(chunk, local_x, local_y, local_z) {
                    continue;
                }
                enqueue_spread_targets_from_fluid(
                    pending,
                    world,
                    origin + IVec3::new(local_x, local_y, local_z),
                );
                remaining_dynamic -= 1;
                if remaining_dynamic == 0 {
                    return;
                }
            }
        }
        return;
    }

    if direction.y != 0 {
        let local_y = if direction.y < 0 { 0 } else { size - 1 };
        for local_z in 0..size {
            for local_x in 0..size {
                if !is_dynamic_fluid(chunk, local_x, local_y, local_z) {
                    continue;
                }
                enqueue_spread_targets_from_fluid(
                    pending,
                    world,
                    origin + IVec3::new(local_x, local_y, local_z),
                );
                remaining_dynamic -= 1;
                if remaining_dynamic == 0 {
                    return;
                }
            }
        }
        return;
    }

    let local_z = if direction.z < 0 { 0 } else { size - 1 };
    for local_y in 0..size {
        for local_x in 0..size {
            if !is_dynamic_fluid(chunk, local_x, local_y, local_z) {
                continue;
            }
            enqueue_spread_targets_from_fluid(
                pending,
                world,
                origin + IVec3::new(local_x, local_y, local_z),
            );
            remaining_dynamic -= 1;
            if remaining_dynamic == 0 {
                return;
            }
        }
    }
}

fn is_dynamic_fluid(chunk: &VoxelChunk, x: i32, y: i32, z: i32) -> bool {
    chunk
        .fluid_at(x, y, z)
        .is_some_and(|fluid| !fluid.is_source())
}

fn enqueue_spread_targets_from_fluid(
    pending: &mut PendingFluidUpdates,
    world: &VoxelWorld,
    position: IVec3,
) {
    for offset in FLUID_SPREAD_TARGETS {
        let target = position + offset;
        let Some((cell, fluid, _)) = world.sample_at(target) else {
            continue;
        };
        if cell.is_some() || fluid.is_some() {
            continue;
        }

        if offset == IVec3::NEG_Y {
            pending.enqueue_priority(target);
        } else {
            pending.enqueue(target);
        }
    }
}
