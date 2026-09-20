use bevy::{
    platform::collections::HashSet,
    prelude::*,
};

use crate::{
    content::fluid::FluidRegistry,
    voxel::{
        coordinates::chunk_coord_from_world,
        deduplicated_queue::DeduplicatedQueue,
        world::VoxelWorld,
    },
    world::work_budget::FrameWorkBudget,
};

use super::{
    frontier::visit_loaded_fluid_frontier_targets,
    solver::{FluidSolverScratch, desired_fluid_with_scratch},
};

/// Finite one-step priming for generated fluid frontiers.
///
/// Worldgen owns the authored source volume. Before first publication we
/// evaluate only the frontier targets that are exposed at priming time and
/// materialize at most that first dynamic step. Newly created flow never
/// re-enqueues its neighbors here: continuation belongs to the normal
/// scheduled runtime solver and therefore preserves spreadSpeed cadence.
#[derive(Default)]
pub(in crate::world) struct GeneratedFluidPriming {
    generated_chunks: HashSet<IVec3>,
    queue: DeduplicatedQueue<IVec3>,
    scratch: FluidSolverScratch,
    active: bool,
}

impl GeneratedFluidPriming {
    pub(in crate::world) fn is_active(&self) -> bool {
        self.active
    }

    pub(in crate::world) fn begin(
        &mut self,
        world: &VoxelWorld,
        coords: impl IntoIterator<Item = IVec3>,
    ) {
        self.generated_chunks.clear();
        self.queue.clear();
        self.active = false;
        self.extend(world, coords);
    }

    /// Add newly generated chunks to the current publication batch. Frontier
    /// traversal includes loaded neighbor seams, but only targets inside the
    /// newly generated set may be mutated by priming.
    pub(in crate::world) fn extend(
        &mut self,
        world: &VoxelWorld,
        coords: impl IntoIterator<Item = IVec3>,
    ) {
        let mut added = Vec::new();
        for coord in coords {
            if coord.y >= 0 && self.generated_chunks.insert(coord) {
                added.push(coord);
            }
        }
        if added.is_empty() {
            return;
        }

        self.active = true;
        added.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));

        let generated_chunks = &self.generated_chunks;
        let queue = &mut self.queue;
        for coord in added {
            visit_loaded_fluid_frontier_targets(
                world,
                coord,
                &mut |_fluid_id, target, priority| {
                    if !generated_chunks.contains(&chunk_coord_from_world(target)) {
                        return;
                    }
                    if priority {
                        queue.enqueue_front(target);
                    } else {
                        queue.enqueue(target);
                    }
                },
            );
        }
    }

    /// Process a finite snapshot of exposed frontier targets. No mutation made
    /// here creates more priming work, so completion is guaranteed once the
    /// queue has been consumed.
    pub(in crate::world) fn process(
        &mut self,
        world: &mut VoxelWorld,
        fluids: &FluidRegistry,
        budget: &mut FrameWorkBudget,
    ) -> bool {
        if !self.active {
            return true;
        }

        while !budget.exhausted() {
            let Some(position) = self.queue.pop() else {
                break;
            };
            budget.record(1);

            if !self
                .generated_chunks
                .contains(&chunk_coord_from_world(position))
            {
                continue;
            }

            let Some((cell, current, _)) = world.sample_at(position) else {
                continue;
            };
            // Priming never overwrites authored/generated fluid, especially a
            // source. It only materializes flow into an empty exposed target.
            if current.is_some() {
                continue;
            }

            let desired = desired_fluid_with_scratch(
                world,
                position,
                cell,
                current,
                fluids,
                &mut self.scratch,
            );
            let Some(desired) = desired else {
                continue;
            };
            if desired.is_source() {
                continue;
            }

            let _ = world.set_derived_fluid_at(position, Some(desired));
        }

        self.queue.len() == 0
    }

    pub(in crate::world) fn take_completed_chunks(&mut self) -> Option<Vec<IVec3>> {
        if !self.active || self.queue.len() != 0 {
            return None;
        }

        let mut completed = self.generated_chunks.drain().collect::<Vec<_>>();
        completed.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        self.active = false;
        Some(completed)
    }
}
