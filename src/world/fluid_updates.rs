mod frontier;
mod solver;

use std::{collections::HashMap, time::Duration};

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
    queue: VoxelUpdateQueue,
    accumulated_steps: HashMap<FluidId, f32>,
}

impl PendingFluidUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.queue.enqueue_with_neighbors_priority(position);
    }

    pub(crate) fn enqueue_loaded_fluid_frontier(&mut self, world: &VoxelWorld, coord: IVec3) {
        frontier::enqueue_loaded_fluid_frontier(self, world, coord);
    }

    fn enqueue(&mut self, position: IVec3) {
        self.queue.enqueue(position);
    }

    fn enqueue_priority(&mut self, position: IVec3) {
        self.queue.enqueue_priority(position);
    }

    fn pop(&mut self) -> Option<IVec3> {
        self.queue.pop()
    }

    fn ready_steps(
        &mut self,
        elapsed_ticks: u32,
        ticks_per_second: u32,
        fluids: &FluidRegistry,
    ) -> HashMap<FluidId, usize> {
        let mut ready = HashMap::new();

        if elapsed_ticks == 0 {
            return ready;
        }

        for (fluid_id, definition) in fluids.iter() {
            let accumulator = self.accumulated_steps.entry(fluid_id).or_default();
            if definition.spread_speed <= f32::EPSILON {
                *accumulator = 0.0;
                continue;
            }

            let steps_per_tick = definition.spread_speed / ticks_per_second as f32;
            *accumulator += elapsed_ticks as f32 * steps_per_tick;
            let elapsed_steps = accumulator.floor() as usize;
            let steps = elapsed_steps.min(MAX_FLUID_STEPS_PER_FRAME);

            if elapsed_steps > 0 {
                *accumulator -= elapsed_steps as f32;
            }
            if steps > 0 {
                ready.insert(fluid_id, steps);
            }
        }

        ready
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
    mut runtime: FluidSimulationRuntime,
) {
    let elapsed_ticks = world_ticks.ticks_this_frame();
    if elapsed_ticks == 0 {
        return;
    }

    let ready_steps = runtime.pending.ready_steps(
        elapsed_ticks,
        game_rules.ticks_per_second(),
        &fluids,
    );
    let max_steps = ready_steps.values().copied().max().unwrap_or(0);
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

        // Freeze the current frontier for this fluid step. Positions enqueued while
        // processing this batch belong to the next step, matching the previous solver
        // semantics even when the temporal budget ends the frame early.
        let batch_len = runtime.pending.queue.len();
        for _ in 0..batch_len {
            if budget.exhausted() {
                break 'steps;
            }

            let Some(position) = runtime.pending.pop() else {
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
            let fluid_ready_steps = ready_steps.get(&fluid_id).copied().unwrap_or(0);

            if fluid_ready_steps <= step_index {
                if definition.spread_speed > f32::EPSILON {
                    runtime.pending.enqueue(position);
                }
                continue;
            }
            if current == desired {
                continue;
            }

            let lighting_medium_changed = current.map(|fluid| fluid.fluid_id)
                != desired.map(|fluid| fluid.fluid_id);

            if runtime.world.set_fluid_at(position, desired).is_none() {
                continue;
            }

            if lighting_medium_changed {
                runtime.lighting.enqueue_medium_edit(position);
            }
            enqueue_remesh(position, &mut runtime.remesh_queue);
            runtime.pending.enqueue_voxel_edit(position);
        }
    }
}
