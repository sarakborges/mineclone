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
    context::LightingContext,
    medium::block_emission,
    propagation::{relax, relax_budgeted},
    queue::LightingQueue,
};
use super::{
    chunk::CHUNK_SIZE,
    coordinates::chunk_origin,
    light::VoxelLight,
    world::VoxelWorld,
};

#[derive(Resource, Default)]
pub(crate) struct PendingLightingUpdates {
    queue: LightingQueue,
    emission_edit_centers: HashSet<IVec3>,
}

impl PendingLightingUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.queue.enqueue_with_neighbors_priority(position);
        self.emission_edit_centers.insert(position);
    }

    pub(crate) fn enqueue_chunk_unloads(&mut self, unloaded: &[IVec3]) {
        for coord in unloaded {
            self.queue.enqueue_chunk_boundary_neighbors(chunk_origin(*coord));
        }
    }

    pub(crate) fn enqueue_chunk_relaxation(&mut self, coord: IVec3) {
        let origin = chunk_origin(coord);
        self.queue.enqueue_chunk_voxels(origin);
        self.queue.enqueue_chunk_boundary_neighbors(origin);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.queue.is_empty() && self.emission_edit_centers.is_empty()
    }

    fn enqueue_emission_edit_volumes(&mut self, world: &VoxelWorld, blocks: &BlockRegistry) {
        let centers = std::mem::take(&mut self.emission_edit_centers);
        let radius = VoxelLight::MAX_LEVEL as i32;

        for center in centers {
            let is_emitter = world
                .block_id_at(center)
                .and_then(|block_id| blocks.get(block_id))
                .is_some_and(|block| block.light_emission > 0);
            if !is_emitter {
                continue;
            }

            for y in -radius..=radius {
                let y_cost = y.abs();
                for z in -radius..=radius {
                    let yz_cost = y_cost + z.abs();
                    if yz_cost > radius {
                        continue;
                    }

                    let x_span = radius - yz_cost;
                    for x in -x_span..=x_span {
                        self.queue.enqueue(center + IVec3::new(x, y, z));
                    }
                }
            }

            self.queue.enqueue_with_neighbors_priority(center);
        }
    }
}

pub(crate) fn seed_chunk_direct_lighting(
    world: &mut VoxelWorld,
    coord: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    secondary_properties: &SecondaryPropertyRegistry,
) {
    let origin = chunk_origin(coord);
    let mut context = LightingContext::default();

    for local_z in 0..CHUNK_SIZE as i32 {
        for local_x in 0..CHUNK_SIZE as i32 {
            for local_y in 0..CHUNK_SIZE as i32 {
                let position = origin + IVec3::new(local_x, local_y, local_z);
                let sky = context.direct_sky_light(
                    world,
                    blocks,
                    fluids,
                    secondary_properties,
                    position,
                );
                let emitted = block_emission(world, blocks, secondary_properties, position);
                world.set_light_at(position, VoxelLight::new_hsi(sky, emitted));
            }
        }
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
    pending.enqueue_emission_edit_volumes(world, blocks);
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
    pending.enqueue_emission_edit_volumes(world, blocks);
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
