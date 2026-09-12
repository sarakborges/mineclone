mod context;
mod medium;
mod propagation;
mod queue;

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use bevy::prelude::*;

use crate::content::{
    block::BlockRegistry, fluid::FluidRegistry,
    secondary_property::SecondaryPropertyRegistry,
};

use self::{
    propagation::{relax, relax_budgeted},
    queue::LightingQueue,
};
use super::{coordinates::chunk_origin, world::VoxelWorld};

#[derive(Resource, Default)]
pub(crate) struct PendingLightingUpdates {
    queue: LightingQueue,
}

impl PendingLightingUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.queue.enqueue_with_neighbors(position);
    }

    pub(crate) fn enqueue_chunk_unloads(&mut self, unloaded: &[IVec3]) {
        for coord in unloaded {
            self.queue.enqueue_chunk_boundary_neighbors(chunk_origin(*coord));
        }
    }

    pub(crate) fn enqueue_chunks_initialization(
        &mut self,
        world: &mut VoxelWorld,
        coords: &[IVec3],
    ) {
        for &coord in coords {
            if !world.clear_chunk_light(coord) {
                continue;
            }

            let origin = chunk_origin(coord);
            self.queue.enqueue_chunk_voxels(origin);
            self.queue.enqueue_chunk_boundary_neighbors(origin);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.queue = LightingQueue::default();
    }
}

pub(crate) fn process_pending_lighting(
    world: &mut VoxelWorld,
    pending: &mut PendingLightingUpdates,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    secondary_properties: &SecondaryPropertyRegistry,
    max_voxels: usize,
) -> HashSet<IVec3> {
    relax_budgeted(
        world,
        blocks,
        fluids,
        secondary_properties,
        &mut pending.queue,
        max_voxels,
    )
}

pub(crate) fn initialize_chunks_lighting(
    world: &mut VoxelWorld,
    coords: &[IVec3],
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    secondary_properties: &SecondaryPropertyRegistry,
) -> HashSet<IVec3> {
    let mut queue = LightingQueue::default();

    for &coord in coords {
        if !world.clear_chunk_light(coord) {
            continue;
        }

        let origin = chunk_origin(coord);
        queue.enqueue_chunk_voxels(origin);
        queue.enqueue_chunk_boundary_neighbors(origin);
    }

    relax(world, blocks, fluids, secondary_properties, &mut queue)
}

#[cfg(test)]
fn initialize_chunk_lighting(
    world: &mut VoxelWorld,
    coord: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> HashSet<IVec3> {
    let secondary_properties = SecondaryPropertyRegistry::default();
    initialize_chunks_lighting(
        world,
        std::slice::from_ref(&coord),
        blocks,
        fluids,
        &secondary_properties,
    )
}

#[cfg(test)]
fn relight_after_voxel_edit(
    world: &mut VoxelWorld,
    position: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) {
    let secondary_properties = SecondaryPropertyRegistry::default();
    let mut pending = PendingLightingUpdates::default();
    pending.enqueue_voxel_edit(position);
    drop(relax(
        world,
        blocks,
        fluids,
        &secondary_properties,
        &mut pending.queue,
    ));
}

#[cfg(test)]
fn relight_after_chunk_unloads(
    world: &mut VoxelWorld,
    unloaded: &[IVec3],
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) {
    let secondary_properties = SecondaryPropertyRegistry::default();
    let mut pending = PendingLightingUpdates::default();
    pending.enqueue_chunk_unloads(unloaded);
    drop(relax(
        world,
        blocks,
        fluids,
        &secondary_properties,
        &mut pending.queue,
    ));
}
