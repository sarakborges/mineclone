use bevy::{
    platform::collections::HashSet,
    prelude::*,
};

use crate::{
    content::fluid::FluidRegistry,
    voxel::{
        coordinates::{chunk_coord_from_world, chunk_origin},
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

#[derive(Debug)]
pub(in crate::world) struct GeneratedFluidSettlingCompletion {
    pub(in crate::world) generated_chunks: Vec<IVec3>,
    pub(in crate::world) changed_existing_positions: Vec<IVec3>,
}

/// Budgeted generated-fluid convergence with explicit fixed-point verification.
///
/// The fast work queue propagates local changes. That queue alone is not a
/// proof of convergence because downhill routing performs a bounded BFS and a
/// mutation may change a distant route preference. Once local work drains, a
/// sparse verification pass evaluates every dynamic fluid cell plus every
/// currently exposed frontier target in the mutable domain. Only a complete
/// verification pass with zero mutations is accepted as a fixed point.
///
/// The mutable domain starts with unpublished generated chunks and may expand
/// through fluid seams into already-resident, non-persistent chunks. This makes
/// generated-fluid settling symmetric across streaming order while preserving
/// player-authored/persistent chunks as runtime-owned state.
#[derive(Default)]
pub(in crate::world) struct GeneratedFluidSettling {
    generated_chunks: HashSet<IVec3>,
    mutable_chunks: HashSet<IVec3>,
    changed_existing_positions: HashSet<IVec3>,
    work_queue: DeduplicatedQueue<IVec3>,
    verification_queue: DeduplicatedQueue<IVec3>,
    verification_chunks: Vec<IVec3>,
    verification_chunk_cursor: usize,
    verification_active: bool,
    verification_changed: bool,
    scratch: FluidSolverScratch,
    active: bool,
    converged: bool,
}

impl GeneratedFluidSettling {
    pub(in crate::world) fn is_active(&self) -> bool {
        self.active
    }

    /// Publication ownership only. Mutable halo chunks are deliberately not
    /// reported here because they may already be rendered.
    pub(in crate::world) fn contains(&self, coord: IVec3) -> bool {
        self.generated_chunks.contains(&coord)
    }

    pub(in crate::world) fn begin(
        &mut self,
        world: &VoxelWorld,
        coords: impl IntoIterator<Item = IVec3>,
    ) {
        self.reset();

        let mut generated = coords
            .into_iter()
            .filter(|coord| coord.y >= 0)
            .collect::<Vec<_>>();
        generated.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        generated.dedup();

        for coord in generated {
            if self.generated_chunks.insert(coord) {
                self.include_mutable_chunk(world, coord, true);
            }
        }

        if self.generated_chunks.is_empty() {
            return;
        }

        self.active = true;
        let mut seeds = self.generated_chunks.iter().copied().collect::<Vec<_>>();
        seeds.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        for coord in seeds {
            self.seed_frontier_into_work(world, coord);
        }
    }

    pub(in crate::world) fn process(
        &mut self,
        world: &mut VoxelWorld,
        fluids: &FluidRegistry,
        budget: &mut FrameWorkBudget,
    ) -> bool {
        if !self.active {
            return true;
        }
        if self.converged {
            return true;
        }

        while !budget.exhausted() {
            if let Some(position) = self.work_queue.pop() {
                budget.record(1);
                self.evaluate_position(world, fluids, position);
                continue;
            }

            if !self.verification_active {
                self.start_verification();
                continue;
            }

            if let Some(position) = self.verification_queue.pop() {
                budget.record(1);
                self.evaluate_position(world, fluids, position);
                continue;
            }

            if let Some(coord) = self
                .verification_chunks
                .get(self.verification_chunk_cursor)
                .copied()
            {
                self.verification_chunk_cursor += 1;
                budget.record(1);
                self.seed_frontier_into_verification(world, coord);
                continue;
            }

            if self.verification_changed {
                self.start_verification();
                continue;
            }

            self.converged = true;
            return true;
        }

        false
    }

    pub(in crate::world) fn take_completion(
        &mut self,
    ) -> Option<GeneratedFluidSettlingCompletion> {
        if !self.active || !self.converged {
            return None;
        }

        let mut generated_chunks = self.generated_chunks.drain().collect::<Vec<_>>();
        generated_chunks.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));

        let mut changed_existing_positions =
            self.changed_existing_positions.drain().collect::<Vec<_>>();
        changed_existing_positions.sort_unstable_by_key(|position| {
            let coord = chunk_coord_from_world(*position);
            (coord.y, coord.z, coord.x, position.y, position.z, position.x)
        });

        self.reset();

        Some(GeneratedFluidSettlingCompletion {
            generated_chunks,
            changed_existing_positions,
        })
    }

    fn reset(&mut self) {
        self.generated_chunks.clear();
        self.mutable_chunks.clear();
        self.changed_existing_positions.clear();
        self.work_queue.clear();
        self.verification_queue.clear();
        self.verification_chunks.clear();
        self.verification_chunk_cursor = 0;
        self.verification_active = false;
        self.verification_changed = false;
        self.active = false;
        self.converged = false;
    }

    fn start_verification(&mut self) {
        self.verification_queue.clear();

        self.verification_chunks.clear();
        self.verification_chunks
            .extend(self.mutable_chunks.iter().copied());
        self.verification_chunks
            .sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        self.verification_chunk_cursor = 0;

        self.verification_active = true;
        self.verification_changed = false;
    }

    fn evaluate_position(
        &mut self,
        world: &mut VoxelWorld,
        fluids: &FluidRegistry,
        position: IVec3,
    ) {
        let coord = chunk_coord_from_world(position);
        let already_mutable = self.mutable_chunks.contains(&coord);

        let Some((cell, current, _)) = world.sample_at(position) else {
            return;
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
            return;
        }

        if !already_mutable && !world.derived_fluid_chunk_is_mutable(coord) {
            return;
        }
        if !already_mutable && !self.include_mutable_chunk(world, coord, false) {
            return;
        }

        if world.set_derived_fluid_at(position, desired).is_none() {
            return;
        }

        if self.verification_active {
            self.verification_changed = true;
        }

        if !self.generated_chunks.contains(&coord) {
            self.changed_existing_positions.insert(position);
        }

        self.enqueue_changed_neighborhood(world, position);
    }

    fn include_mutable_chunk(
        &mut self,
        world: &VoxelWorld,
        coord: IVec3,
        generated: bool,
    ) -> bool {
        if self.mutable_chunks.contains(&coord) {
            return true;
        }
        if !generated && !world.derived_fluid_chunk_is_mutable(coord) {
            return false;
        }
        if world.chunk(coord).is_none() {
            return false;
        }

        self.mutable_chunks.insert(coord);

        if self.verification_active {
            self.verification_changed = true;
        }
        true
    }

    fn enqueue_work_target(&mut self, world: &VoxelWorld, target: IVec3, priority: bool) {
        if target.y < 0 || world.sample_at(target).is_none() {
            return;
        }
        if priority {
            self.work_queue.enqueue_front(target);
        } else {
            self.work_queue.enqueue(target);
        }
    }

    fn enqueue_changed_neighborhood(&mut self, world: &VoxelWorld, position: IVec3) {
        self.enqueue_work_target(world, position, false);
        self.enqueue_work_target(world, position - IVec3::Y, true);
        for offset in HORIZONTAL_NEIGHBORS {
            self.enqueue_work_target(world, position + offset, false);
        }
    }

    fn seed_frontier_into_work(&mut self, world: &VoxelWorld, coord: IVec3) {
        let mut targets = Vec::new();
        visit_loaded_fluid_frontier_targets(
            world,
            coord,
            &mut |_fluid_id, target, priority| targets.push((target, priority)),
        );
        for (target, priority) in targets {
            self.enqueue_work_target(world, target, priority);
        }
    }

    fn seed_frontier_into_verification(&mut self, world: &VoxelWorld, coord: IVec3) {
        if !self.mutable_chunks.contains(&coord) || world.chunk(coord).is_none() {
            return;
        }

        let origin = chunk_origin(coord);
        if let Some(chunk) = world.chunk(coord) {
            chunk.visit_dynamic_fluid_cells(|local_position, _fluid| {
                self.verification_queue.enqueue(origin + local_position);
            });
        }

        let mut targets = Vec::new();
        visit_loaded_fluid_frontier_targets(
            world,
            coord,
            &mut |_fluid_id, target, priority| targets.push((target, priority)),
        );

        for (target, priority) in targets {
            if target.y < 0 || world.sample_at(target).is_none() {
                continue;
            }
            if priority {
                self.verification_queue.enqueue_front(target);
            } else {
                self.verification_queue.enqueue(target);
            }
        }
    }
}
