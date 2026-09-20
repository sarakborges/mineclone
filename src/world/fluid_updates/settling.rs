use bevy::{
    platform::collections::HashSet,
    prelude::*,
};

use crate::{
    content::fluid::FluidRegistry,
    voxel::{
        coordinates::chunk_coord_from_world,
        deduplicated_queue::DeduplicatedQueue,
        neighbors::HORIZONTAL_NEIGHBORS,
        world::VoxelWorld,
    },
    world::work_budget::FrameWorkBudget,
};

use super::{
    frontier::visit_loaded_fluid_frontier_targets,
    solver::{FluidSolverScratch, desired_fluid_with_scratch},
};

/// Incremental convergence state for generated fluid frontiers.
///
/// This is worldgen convergence, not gameplay simulation: authored spread
/// cadence is intentionally ignored so a chunk's first published mesh already
/// contains its initial settled fluid state. Work is retained across frames so
/// large oceans/waterfalls can never monopolize a loading or gameplay frame.
#[derive(Default)]
pub(in crate::world) struct GeneratedFluidSettling {
    generated_chunks: HashSet<IVec3>,
    queue: DeduplicatedQueue<IVec3>,
    scratch: FluidSolverScratch,
    active: bool,
}

impl GeneratedFluidSettling {
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

    /// Extend the current mutable worldgen set before publication. Targets
    /// outside this set are intentionally left to the runtime scheduler.
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

    /// Spend only the caller-provided frame budget. Returns true when the
    /// current generated set has converged.
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
            let desired = desired_fluid_with_scratch(
                world,
                position,
                cell,
                current,
                fluids,
                &mut self.scratch,
            );
            if current == desired {
                continue;
            }

            if world.set_derived_fluid_at(position, desired).is_none() {
                continue;
            }

            enqueue_changed_neighborhood(
                &mut self.queue,
                position,
                &self.generated_chunks,
            );
        }

        self.queue.len() == 0
    }

    /// Drain the converged chunk set exactly once. Callers use this as the
    /// publication barrier before lighting/first mesh.
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

fn enqueue_changed_neighborhood(
    queue: &mut DeduplicatedQueue<IVec3>,
    position: IVec3,
    generated_chunks: &HashSet<IVec3>,
) {
    for target in std::iter::once(position)
        .chain(std::iter::once(position - IVec3::Y))
        .chain(HORIZONTAL_NEIGHBORS.into_iter().map(|offset| position + offset))
    {
        if target.y >= 0 && generated_chunks.contains(&chunk_coord_from_world(target)) {
            queue.enqueue(target);
        }
    }
}
