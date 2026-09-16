mod context;
mod medium;
mod propagation;
mod queue;

#[cfg(test)]
mod tests;

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::content::{
    block::BlockRegistry, fluid::FluidRegistry, secondary_property::SecondaryPropertyRegistry,
};

use self::{
    context::LightingContext,
    medium::{block_emission_for_cell, medium_dampening_for_cells},
    propagation::{LightingRegistries, relax, relax_budgeted},
    queue::LightingQueue,
};
use super::{
    cell::VoxelCell,
    chunk::CHUNK_SIZE,
    coordinates::chunk_origin,
    light::{BlockLight, VoxelLight},
    world::VoxelWorld,
};

#[derive(Resource, Default)]
pub(crate) struct PendingLightingUpdates {
    queue: LightingQueue,
    emission_edit_previous_cells: HashMap<IVec3, Option<VoxelCell>>,
    context: LightingContext,
    interactive_changed_chunks: HashSet<IVec3>,
}

impl PendingLightingUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3, previous_cell: Option<VoxelCell>) {
        self.queue.enqueue_with_neighbors_priority(position);
        self.emission_edit_previous_cells
            .entry(position)
            .or_insert(previous_cell);
    }

    pub(crate) fn enqueue_medium_edit(&mut self, position: IVec3) {
        self.queue.enqueue_with_neighbors(position);
    }

    pub(crate) fn enqueue_chunk_unloads(&mut self, unloaded: &[IVec3]) {
        for coord in unloaded {
            self.queue
                .enqueue_chunk_boundary_neighbors(chunk_origin(*coord));
        }
    }

    pub(crate) fn enqueue_chunk_relaxation(&mut self, coord: IVec3) {
        let origin = chunk_origin(coord);
        self.queue.enqueue_chunk_voxels(origin);
        self.queue.enqueue_chunk_boundary_neighbors(origin);
    }

    pub(crate) fn enqueue_empty_chunk_relaxation(&mut self, coord: IVec3) {
        let origin = chunk_origin(coord);
        self.queue.enqueue_chunk_boundary_voxels(origin);
        self.queue.enqueue_chunk_boundary_neighbors(origin);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.queue.is_empty()
            && self.emission_edit_previous_cells.is_empty()
            && self.interactive_changed_chunks.is_empty()
    }

    fn enqueue_emission_edit_volumes(
        &mut self,
        world: &VoxelWorld,
        blocks: &BlockRegistry,
        secondary_properties: &SecondaryPropertyRegistry,
    ) {
        let radius = VoxelLight::MAX_LEVEL as i32;
        let queue = &mut self.queue;

        for (center, previous_cell) in self.emission_edit_previous_cells.drain() {
            let previous_emission =
                block_emission_for_cell(previous_cell, blocks, secondary_properties);
            let current_emission =
                block_emission_for_cell(world.cell_at(center), blocks, secondary_properties);
            if !emission_change_requires_full_volume(previous_emission, current_emission) {
                continue;
            }

            // Changing an existing source's HSI emission can leave stale color channels
            // mutually supporting one another in the incremental field. Re-evaluate the
            // complete maximum Manhattan footprint as interactive work so source edits
            // cannot fall behind streaming relaxation. New and removed sources converge
            // through the same interactive propagation lane.
            for y in -radius..=radius {
                let y_cost = y.abs();
                for z in -radius..=radius {
                    let yz_cost = y_cost + z.abs();
                    if yz_cost > radius {
                        continue;
                    }

                    let x_span = radius - yz_cost;
                    for x in -x_span..=x_span {
                        queue.enqueue_interactive(center + IVec3::new(x, y, z));
                    }
                }
            }

            queue.enqueue_with_neighbors_priority(center);
        }
    }
}

fn emission_change_requires_full_volume(previous: BlockLight, current: BlockLight) -> bool {
    previous != current && previous.intensity() > 0 && current.intensity() > 0
}

pub(crate) fn seed_chunk_direct_lighting(
    world: &mut VoxelWorld,
    coord: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    secondary_properties: &SecondaryPropertyRegistry,
) {
    let size = CHUNK_SIZE as i32;
    let world_x = coord.x * size;
    let world_z = coord.z * size;
    let highest_loaded_chunk_y = world
        .highest_loaded_world_y_in_column(world_x, world_z)
        .map(|highest_y| highest_y.div_euclid(size))
        .unwrap_or(coord.y);
    debug_assert!(highest_loaded_chunk_y >= coord.y);

    let mut sky_by_column = [VoxelLight::MAX_LEVEL; CHUNK_SIZE * CHUNK_SIZE];
    for upper_y in ((coord.y + 1)..=highest_loaded_chunk_y).rev() {
        let Some(upper_chunk) = world.chunk(IVec3::new(coord.x, upper_y, coord.z)) else {
            continue;
        };
        if upper_chunk.is_empty() {
            continue;
        }

        for local_z in 0..CHUNK_SIZE {
            for local_x in 0..CHUNK_SIZE {
                let sky = &mut sky_by_column[local_x + local_z * CHUNK_SIZE];
                if *sky == 0 {
                    continue;
                }

                for local_y in (0..CHUNK_SIZE).rev() {
                    if *sky == 0 {
                        break;
                    }
                    let (cell, fluid, _) = upper_chunk
                        .sample_local(local_x as i32, local_y as i32, local_z as i32)
                        .expect("direct seed local coordinates must stay inside the chunk");
                    *sky =
                        sky.saturating_sub(medium_dampening_for_cells(cell, fluid, blocks, fluids));
                }
            }
        }
    }

    if world.chunk(coord).is_some_and(|chunk| chunk.is_empty()) {
        let seeded = world.rebuild_empty_chunk_light_columns(coord, &sky_by_column);
        debug_assert!(seeded, "seeded chunk must be loaded: {coord:?}");
        return;
    }

    let seeded = world.rebuild_chunk_light(coord, |x, _, z, cell, fluid| {
        let sky = &mut sky_by_column[x + z * CHUNK_SIZE];
        if *sky > 0 {
            *sky = sky.saturating_sub(medium_dampening_for_cells(cell, fluid, blocks, fluids));
        }
        let emitted = block_emission_for_cell(cell, blocks, secondary_properties);
        VoxelLight::new_hsi(*sky, emitted)
    });
    debug_assert!(seeded, "seeded chunk must be loaded: {coord:?}");
}

pub(crate) fn process_pending_lighting(
    world: &mut VoxelWorld,
    pending: &mut PendingLightingUpdates,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    secondary_properties: &SecondaryPropertyRegistry,
    changed_chunks: &mut HashSet<IVec3>,
    budget_exhausted: impl FnMut(usize) -> bool,
) {
    pending.enqueue_emission_edit_volumes(world, blocks, secondary_properties);
    let PendingLightingUpdates {
        queue,
        context,
        interactive_changed_chunks,
        ..
    } = pending;
    relax_budgeted(
        world,
        LightingRegistries::new(blocks, fluids, secondary_properties),
        queue,
        context,
        changed_chunks,
        interactive_changed_chunks,
        budget_exhausted,
    );
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
    pending.enqueue_voxel_edit(position, None);
    pending.enqueue_emission_edit_volumes(world, blocks, &secondary_properties);
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
