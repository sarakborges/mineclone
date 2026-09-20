use bevy::prelude::*;

use crate::{
    voxel::{
        chunk::CHUNK_SIZE,
        neighbors::CARDINAL_NEIGHBORS,
        world::VoxelWorld,
    },
};

use super::{PendingFluidUpdates, solver::can_spread_horizontally_from};

const FLUID_SPREAD_TARGETS: [IVec3; 5] = [
    IVec3::NEG_Y,
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Z,
    IVec3::NEG_Z,
];

pub(super) fn enqueue_resident_fluid_frontier(
    pending: &mut PendingFluidUpdates,
    world: &VoxelWorld,
    coord: IVec3,
) {
    enqueue_chunk_fluid_spread_targets(pending, world, coord);
}

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

    let origin = coord * CHUNK_SIZE as i32;
    chunk.visit_potential_fluid_frontier_sources(|local_position, fluid| {
        enqueue_spread_targets_from_fluid(
            pending,
            world,
            origin + local_position,
            fluid,
        );
    });
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
                let Some(fluid) = chunk.fluid_at(local_x, local_y, local_z) else {
                    continue;
                };
                enqueue_spread_targets_from_fluid(
                    pending,
                    world,
                    origin + IVec3::new(local_x, local_y, local_z),
                    fluid,
                );
            }
        }
        return;
    }

    if direction.y != 0 {
        let local_y = if direction.y < 0 { 0 } else { size - 1 };
        for local_z in 0..size {
            for local_x in 0..size {
                let Some(fluid) = chunk.fluid_at(local_x, local_y, local_z) else {
                    continue;
                };
                enqueue_spread_targets_from_fluid(
                    pending,
                    world,
                    origin + IVec3::new(local_x, local_y, local_z),
                    fluid,
                );
            }
        }
        return;
    }

    let local_z = if direction.z < 0 { 0 } else { size - 1 };
    for local_y in 0..size {
        for local_x in 0..size {
            let Some(fluid) = chunk.fluid_at(local_x, local_y, local_z) else {
                continue;
            };
            enqueue_spread_targets_from_fluid(
                pending,
                world,
                origin + IVec3::new(local_x, local_y, local_z),
                fluid,
            );
        }
    }
}

fn enqueue_spread_targets_from_fluid(
    pending: &mut PendingFluidUpdates,
    world: &VoxelWorld,
    position: IVec3,
    source_fluid: crate::voxel::fluid::FluidCell,
) {
    let mut can_spread_horizontally = None;

    for offset in FLUID_SPREAD_TARGETS {
        let target = position + offset;
        let Some((cell, target_fluid, _)) = world.sample_at(target) else {
            continue;
        };
        if cell.is_some() || target_fluid.is_some() {
            continue;
        }

        if offset != IVec3::NEG_Y {
            let eligible = *can_spread_horizontally.get_or_insert_with(|| {
                can_spread_horizontally_from(world, position, source_fluid)
            });
            if !eligible {
                continue;
            }
        }

        if offset == IVec3::NEG_Y {
            pending.enqueue_fluid_priority(source_fluid.fluid_id, target);
        } else {
            pending.enqueue_fluid(source_fluid.fluid_id, target);
        }
    }
}
