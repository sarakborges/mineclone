use bevy::prelude::*;

use crate::voxel::{
    chunk::{CHUNK_SIZE, VoxelChunk},
    neighbors::CARDINAL_NEIGHBORS,
    world::VoxelWorld,
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
    // Generated fluids are the initial world state, not permanently static
    // decoration. Seed only their currently empty neighboring targets into the
    // runtime solver; filled fluid cells themselves do not need queue entries.
    enqueue_chunk_fluid_spread_targets(pending, world, coord);

    // A neighboring chunk may have loaded earlier while this chunk was absent.
    // Revisit every fluid on the shared loaded boundaries so sources and
    // dynamic flow can cross the seam as soon as the target chunk exists.
    for offset in CARDINAL_NEIGHBORS {
        enqueue_neighbor_boundary_spread_targets(pending, world, coord + offset, -offset);
    }
}

fn enqueue_chunk_fluid_spread_targets(
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
    if !chunk.has_fluid() {
        return;
    }

    let size = CHUNK_SIZE as i32;
    let origin = coord * size;

    for local_y in 0..size {
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
    if !chunk.boundary_has_fluid(direction) {
        return;
    }

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
