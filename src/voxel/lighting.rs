mod context;
mod medium;
mod propagation;
mod queue;

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use bevy::prelude::*;

use crate::content::{block::BlockRegistry, fluid::FluidRegistry};

use self::{
    propagation::{relax, relax_budgeted},
    queue::LightingQueue,
};
use super::{chunk::CHUNK_SIZE, world::VoxelWorld};

#[derive(Resource, Default)]
pub(crate) struct PendingLightingUpdates {
    queue: LightingQueue,
}

impl PendingLightingUpdates {
    pub fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.queue.enqueue(position);
        self.queue.enqueue_neighbors(position);
    }

    pub fn enqueue_chunk_unloads(&mut self, unloaded: &[IVec3]) {
        let chunk_size = CHUNK_SIZE as i32;

        for coord in unloaded {
            self.queue
                .enqueue_chunk_boundary_neighbors(*coord * chunk_size);
        }
    }

    pub fn clear(&mut self) {
        self.queue = LightingQueue::default();
    }
}

pub(crate) fn process_pending_lighting(
    world: &mut VoxelWorld,
    pending: &mut PendingLightingUpdates,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    max_voxels: usize,
) -> HashSet<IVec3> {
    relax_budgeted(world, blocks, fluids, &mut pending.queue, max_voxels)
}

pub(crate) fn initialize_chunk_lighting(
    world: &mut VoxelWorld,
    coord: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> HashSet<IVec3> {
    if !world.clear_chunk_light(coord) {
        return HashSet::new();
    }

    let mut queue = LightingQueue::default();
    let origin = coord * CHUNK_SIZE as i32;
    queue.enqueue_chunk_voxels(origin);
    queue.enqueue_chunk_boundary_neighbors(origin);

    relax(world, blocks, fluids, &mut queue)
}

pub(crate) fn initialize_chunks_lighting(
    world: &mut VoxelWorld,
    coords: &[IVec3],
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) {
    let mut queue = LightingQueue::default();
    let chunk_size = CHUNK_SIZE as i32;

    for &coord in coords {
        if !world.clear_chunk_light(coord) {
            continue;
        }

        let origin = coord * chunk_size;
        queue.enqueue_chunk_voxels(origin);
        queue.enqueue_chunk_boundary_neighbors(origin);
    }

    drop(relax(world, blocks, fluids, &mut queue));
}
