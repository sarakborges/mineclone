use bevy::{
    platform::collections::{HashMap, HashSet},
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
    verification_revisions: HashMap<IVec3, u64>,
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
        fluids: &FluidRegistry,
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
            self.seed_dependency_halo(world, fluids, coord);
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
                self.start_verification(world);
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

            if self.verification_changed || self.verification_inputs_changed(world) {
                self.start_verification(world);
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
        self.verification_revisions.clear();
        self.verification_active = false;
        self.verification_changed = false;
        self.active = false;
        self.converged = false;
    }

    fn start_verification(&mut self, world: &VoxelWorld) {
        self.verification_queue.clear();

        self.verification_chunks.clear();
        self.verification_chunks
            .extend(self.mutable_chunks.iter().copied());
        self.verification_chunks
            .sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        self.verification_chunk_cursor = 0;

        self.verification_revisions.clear();
        for &coord in &self.verification_chunks {
            if let Some(revision) = world.chunk_content_revision(coord) {
                self.verification_revisions.insert(coord, revision);
            }
        }

        self.verification_active = true;
        self.verification_changed = false;
    }

    fn verification_inputs_changed(&self, world: &VoxelWorld) -> bool {
        self.verification_revisions.iter().any(|(coord, revision)| {
            world.chunk_content_revision(*coord) != Some(*revision)
        })
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

    fn seed_dependency_halo(
        &mut self,
        world: &VoxelWorld,
        fluids: &FluidRegistry,
        generated_coord: IVec3,
    ) {
        for y in -1..=1 {
            for z in -1..=1 {
                for x in -1..=1 {
                    if x == 0 && y == 0 && z == 0 {
                        continue;
                    }

                    let neighbor = generated_coord + IVec3::new(x, y, z);
                    if self.generated_chunks.contains(&neighbor)
                        || !world.derived_fluid_chunk_is_mutable(neighbor)
                    {
                        continue;
                    }
                    let Some(chunk) = world.chunk(neighbor) else {
                        continue;
                    };
                    let origin = chunk_origin(neighbor);

                    chunk.visit_dynamic_fluid_cells(|local_position, fluid| {
                        let Some(definition) = fluids.get(fluid.fluid_id) else {
                            return;
                        };
                        let position = origin + local_position;
                        if fluid_dependency_reaches_chunk(
                            position,
                            definition.max_spread,
                            generated_coord,
                            y,
                        ) {
                            self.work_queue.enqueue(position);
                        }
                    });

                    // Same-level neighboring sources may have empty targets whose
                    // downhill preference changed when the new chunk became resident.
                    // Direct vertical seam frontiers are already covered by
                    // seed_frontier_into_work(generated_coord), which revisits all
                    // six cardinal neighbor boundaries.
                    if y != 0 {
                        continue;
                    }

                    let mut frontier_targets = Vec::new();
                    visit_loaded_fluid_frontier_targets(
                        world,
                        neighbor,
                        &mut |fluid_id, target, priority| {
                            let Some(definition) = fluids.get(fluid_id) else {
                                return;
                            };
                            if fluid_dependency_reaches_chunk(
                                target,
                                definition.max_spread,
                                generated_coord,
                                0,
                            ) {
                                frontier_targets.push((target, priority));
                            }
                        },
                    );
                    for (target, priority) in frontier_targets {
                        self.enqueue_work_target(world, target, priority);
                    }
                }
            }
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
 
fn fluid_dependency_reaches_chunk(
    position: IVec3,
    max_spread: u16,
    coord: IVec3,
    vertical_offset: i32,
) -> bool {
    let minimum = chunk_origin(coord);
    let maximum = minimum + IVec3::splat(crate::voxel::chunk::CHUNK_SIZE as i32 - 1);
    let touches_relevant_vertical_plane = match vertical_offset {
        -1 => position.y == minimum.y - 1,
        0 => position.y >= minimum.y && position.y <= maximum.y,
        1 => position.y == maximum.y + 1,
        _ => false,
    };
    touches_relevant_vertical_plane
        && horizontal_distance_to_chunk(position, minimum, maximum)
            <= i32::from(max_spread).saturating_add(1)
}

fn horizontal_distance_to_chunk(position: IVec3, minimum: IVec3, maximum: IVec3) -> i32 {
    let x = if position.x < minimum.x {
        minimum.x - position.x
    } else if position.x > maximum.x {
        position.x - maximum.x
    } else {
        0
    };
    let z = if position.z < minimum.z {
        minimum.z - position.z
    } else if position.z > maximum.z {
        position.z - maximum.z
    } else {
        0
    };
    x.saturating_add(z)
}

