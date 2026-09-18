mod frontier;
mod solver;

use std::time::Duration;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::fluid::{FluidId, FluidRegistry},
    voxel::{
        lighting::PendingLightingUpdates, update_queue::VoxelUpdateQueue, world::VoxelWorld,
    },
};

use self::solver::{desired_fluid, enqueue_remesh};
use super::{
    chunk_remesh::ChunkRemeshQueue,
    game_rules::GameRules,
    tick::WorldTickClock,
    work_budget::FrameWorkBudget,
};

const FLUID_UPDATE_BUDGET: Duration = Duration::from_millis(1);
const MIN_FLUID_UPDATES_BEFORE_BUDGET_CHECK: usize = 64;
const MAX_FLUID_UPDATES_PER_FRAME: usize = 512;
const MAX_FLUID_STEPS_PER_FRAME: usize = 4;

#[derive(Resource, Default)]
pub(crate) struct PendingFluidUpdates {
    // Topology edits do not know which neighboring fluid will win, so they
    // enter this generic queue and are classified when the solver inspects the
    // current world state.
    queue: VoxelUpdateQueue,
    // Generated/frontier work and generic positions waiting for a slower fluid
    // live in per-fluid queues. A faster fluid must never push a slower one to
    // the back of one global queue.
    fluid_queues: Vec<VoxelUpdateQueue>,
    accumulated_steps: Vec<f32>,
    next_fluid_queue: usize,
}

impl PendingFluidUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.queue.enqueue_with_neighbors_priority(position);
    }

    pub(crate) fn enqueue_loaded_fluid_frontier(&mut self, world: &VoxelWorld, coord: IVec3) {
        frontier::enqueue_loaded_fluid_frontier(self, world, coord);
    }

    fn enqueue_fluid(&mut self, fluid_id: FluidId, position: IVec3) {
        self.fluid_queue_mut(fluid_id).enqueue(position);
    }

    fn enqueue_fluid_priority(&mut self, fluid_id: FluidId, position: IVec3) {
        self.fluid_queue_mut(fluid_id).enqueue_priority(position);
    }

    fn enqueue_fluid_voxel_edit(&mut self, fluid_id: FluidId, position: IVec3) {
        self.fluid_queue_mut(fluid_id)
            .enqueue_with_neighbors_priority(position);
    }

    fn fluid_queue_mut(&mut self, fluid_id: FluidId) -> &mut VoxelUpdateQueue {
        let index = fluid_id as usize;
        if self.fluid_queues.len() <= index {
            self.fluid_queues
                .resize_with(index + 1, VoxelUpdateQueue::default);
        }
        &mut self.fluid_queues[index]
    }

    fn pop(&mut self) -> Option<IVec3> {
        self.queue.pop()
    }

    fn snapshot_ready_fluid_work(
        &self,
        ready_steps: &[usize],
        step_index: usize,
        remaining: &mut Vec<usize>,
    ) {
        remaining.clear();
        remaining.extend(self.fluid_queues.iter().enumerate().map(|(index, queue)| {
            if ready_steps
                .get(index)
                .is_some_and(|steps| *steps > step_index)
            {
                queue.len()
            } else {
                0
            }
        }));
    }

    fn pop_ready_fluid(&mut self, remaining: &mut [usize]) -> Option<IVec3> {
        let queue_count = remaining.len();
        if queue_count == 0 {
            return None;
        }

        for offset in 0..queue_count {
            let index = (self.next_fluid_queue + offset) % queue_count;
            if remaining[index] == 0 {
                continue;
            }

            remaining[index] -= 1;
            self.next_fluid_queue = (index + 1) % queue_count;
            let position = self
                .fluid_queues
                .get_mut(index)
                .and_then(VoxelUpdateQueue::pop);
            if position.is_some() {
                return position;
            }

            remaining[index] = 0;
        }

        None
    }

    fn update_ready_steps(
        &mut self,
        elapsed_ticks: u32,
        ticks_per_second: u32,
        fluids: &FluidRegistry,
        ready_steps: &mut Vec<usize>,
    ) -> usize {
        ready_steps.clear();

        if elapsed_ticks == 0 {
            return 0;
        }

        let mut max_steps = 0;
        for (fluid_id, definition) in fluids.iter() {
            let index = fluid_id as usize;
            if self.accumulated_steps.len() <= index {
                self.accumulated_steps.resize(index + 1, 0.0);
            }
            if self.fluid_queues.len() <= index {
                self.fluid_queues
                    .resize_with(index + 1, VoxelUpdateQueue::default);
            }

            let accumulator = &mut self.accumulated_steps[index];
            if definition.spread_speed <= f32::EPSILON {
                *accumulator = 0.0;
                ready_steps.push(0);
                continue;
            }

            let steps_per_tick = definition.spread_speed / ticks_per_second as f32;
            *accumulator += elapsed_ticks as f32 * steps_per_tick;
            let elapsed_steps = accumulator.floor() as usize;
            let steps = elapsed_steps.min(MAX_FLUID_STEPS_PER_FRAME);

            if elapsed_steps > 0 {
                *accumulator -= elapsed_steps as f32;
            }

            ready_steps.push(steps);
            max_steps = max_steps.max(steps);
        }

        max_steps
    }
}

#[derive(SystemParam)]
pub(super) struct FluidSimulationRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
    pending: ResMut<'w, PendingFluidUpdates>,
    lighting: ResMut<'w, PendingLightingUpdates>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
}

pub(super) fn process_fluid_updates(
    world_ticks: Res<WorldTickClock>,
    game_rules: Res<GameRules>,
    fluids: Res<FluidRegistry>,
    mut ready_steps: Local<Vec<usize>>,
    mut fluid_batch_remaining: Local<Vec<usize>>,
    mut runtime: FluidSimulationRuntime,
) {
    let elapsed_ticks = world_ticks.ticks_this_frame();
    if elapsed_ticks == 0 {
        return;
    }

    let max_steps = runtime.pending.update_ready_steps(
        elapsed_ticks,
        game_rules.ticks_per_second(),
        &fluids,
        &mut ready_steps,
    );
    if max_steps == 0 {
        return;
    }

    let mut budget = FrameWorkBudget::new(
        FLUID_UPDATE_BUDGET,
        MIN_FLUID_UPDATES_BEFORE_BUDGET_CHECK,
    )
    .with_maximum_items(MAX_FLUID_UPDATES_PER_FRAME);

    'steps: for step_index in 0..max_steps {
        if budget.exhausted() {
            break;
        }

        // Freeze both the generic topology frontier and each ready fluid
        // frontier for this simulation step. Work produced while solving the
        // batch belongs to the next step.
        let mut generic_remaining = runtime.pending.queue.len();
        runtime.pending.snapshot_ready_fluid_work(
            &ready_steps,
            step_index,
            &mut fluid_batch_remaining,
        );

        loop {
            if budget.exhausted() {
                break 'steps;
            }

            let position = if generic_remaining > 0 {
                generic_remaining -= 1;
                let Some(position) = runtime.pending.pop() else {
                    break;
                };
                position
            } else if let Some(position) = runtime
                .pending
                .pop_ready_fluid(&mut fluid_batch_remaining)
            {
                position
            } else {
                break;
            };
            budget.record(1);

            let Some((cell, current, _)) = runtime.world.sample_at(position) else {
                continue;
            };

            let desired = desired_fluid(&runtime.world, position, cell, current, &fluids);
            let Some(fluid_id) = current.or(desired).map(|fluid| fluid.fluid_id) else {
                continue;
            };
            let definition = fluids
                .get(fluid_id)
                .unwrap_or_else(|| panic!("missing fluid definition for id {fluid_id}"));
            let fluid_ready_steps = ready_steps.get(fluid_id as usize).copied().unwrap_or(0);

            if fluid_ready_steps <= step_index {
                if definition.spread_speed > f32::EPSILON {
                    runtime.pending.enqueue_fluid_priority(fluid_id, position);
                }
                continue;
            }
            if current == desired {
                continue;
            }

            if runtime.world.set_fluid_at(position, desired).is_none() {
                continue;
            }

            runtime.lighting.enqueue_medium_edit(position);
            enqueue_remesh(position, &mut runtime.remesh_queue);
            runtime
                .pending
                .enqueue_fluid_voxel_edit(fluid_id, position);
        }
    }
}
