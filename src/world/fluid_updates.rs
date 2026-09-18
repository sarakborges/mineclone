mod frontier;
mod solver;

use std::time::Duration;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::fluid::{FluidId, FluidRegistry},
    voxel::{
        fluid::FluidCell, lighting::PendingLightingUpdates, update_queue::VoxelUpdateQueue,
        world::VoxelWorld,
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
const MAX_QUEUED_FLUID_STEPS: usize = 4;

#[derive(Debug, Default)]
struct FluidUpdateLane {
    queue: VoxelUpdateQueue,
    accumulated_steps: f32,
    ready_steps: usize,
    batch_remaining: usize,
}

#[derive(Resource, Default)]
pub(crate) struct PendingFluidUpdates {
    // Runtime block edits are classified against the current voxel state before
    // they enter a fluid-specific cadence lane.
    topology_queue: VoxelUpdateQueue,
    fluid_lanes: Vec<FluidUpdateLane>,
    next_fluid_lane: usize,
}

impl PendingFluidUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.topology_queue
            .enqueue_with_neighbors_priority(position);
    }

    pub(crate) fn enqueue_loaded_fluid_frontier(&mut self, world: &VoxelWorld, coord: IVec3) {
        frontier::enqueue_loaded_fluid_frontier(self, world, coord);
    }

    fn enqueue_fluid(&mut self, fluid_id: FluidId, position: IVec3) {
        self.fluid_lane_mut(fluid_id).queue.enqueue(position);
    }

    fn enqueue_fluid_priority(&mut self, fluid_id: FluidId, position: IVec3) {
        let lane = self.fluid_lane_mut(fluid_id);
        if lane.batch_remaining == 0 {
            lane.queue.enqueue_priority(position);
        } else {
            // A logical fluid step freezes its frontier. Work discovered after
            // that snapshot belongs to the next step even when it is urgent.
            lane.queue.enqueue(position);
        }
    }

    fn enqueue_fluid_neighborhood(&mut self, fluid_id: FluidId, position: IVec3) {
        self.fluid_lane_mut(fluid_id)
            .queue
            .enqueue_with_neighbors(position);
    }

    fn fluid_lane_mut(&mut self, fluid_id: FluidId) -> &mut FluidUpdateLane {
        let index = fluid_id as usize;
        if self.fluid_lanes.len() <= index {
            self.fluid_lanes
                .resize_with(index + 1, FluidUpdateLane::default);
        }
        &mut self.fluid_lanes[index]
    }

    fn pop_topology(&mut self) -> Option<IVec3> {
        self.topology_queue.pop()
    }

    fn advance_cadence(
        &mut self,
        elapsed_ticks: u32,
        ticks_per_second: u32,
        fluids: &FluidRegistry,
    ) {
        if elapsed_ticks == 0 {
            return;
        }

        for (fluid_id, definition) in fluids.iter() {
            let lane = self.fluid_lane_mut(fluid_id);
            if definition.spread_speed <= f32::EPSILON {
                lane.accumulated_steps = 0.0;
                lane.ready_steps = 0;
                lane.batch_remaining = 0;
                continue;
            }

            let steps_per_tick = definition.spread_speed / ticks_per_second as f32;
            lane.accumulated_steps += elapsed_ticks as f32 * steps_per_tick;
            let elapsed_steps = lane.accumulated_steps.floor() as usize;
            if elapsed_steps == 0 {
                continue;
            }

            lane.accumulated_steps -= elapsed_steps as f32;
            lane.ready_steps = lane
                .ready_steps
                .saturating_add(elapsed_steps)
                .min(MAX_QUEUED_FLUID_STEPS);
        }
    }

    fn pop_runnable_fluid(&mut self) -> Option<(FluidId, IVec3)> {
        let lane_count = self.fluid_lanes.len();
        if lane_count == 0 {
            return None;
        }

        for offset in 0..lane_count {
            let index = (self.next_fluid_lane + offset) % lane_count;
            let lane = &mut self.fluid_lanes[index];

            if lane.batch_remaining == 0 {
                if lane.ready_steps == 0 {
                    continue;
                }
                if lane.queue.len() == 0 {
                    lane.ready_steps = 0;
                    continue;
                }

                lane.batch_remaining = lane.queue.len();
                lane.ready_steps -= 1;
            }

            let Some(position) = lane.queue.pop() else {
                lane.batch_remaining = 0;
                continue;
            };

            lane.batch_remaining -= 1;
            self.next_fluid_lane = (index + 1) % lane_count;
            return Some((index as FluidId, position));
        }

        None
    }

    fn discard_idle_step_credit(&mut self) {
        for lane in &mut self.fluid_lanes {
            if lane.batch_remaining == 0 && lane.queue.len() == 0 {
                lane.ready_steps = 0;
            }
        }
    }
}

pub(super) fn reseed_loaded_fluid_frontiers(
    world: Res<VoxelWorld>,
    mut pending: ResMut<PendingFluidUpdates>,
) {
    *pending = PendingFluidUpdates::default();

    let mut loaded = world.loaded_chunk_coords().collect::<Vec<_>>();
    loaded.sort_by_key(|coord| (coord.y, coord.z, coord.x));
    for coord in loaded {
        frontier::enqueue_resident_fluid_frontier(&mut pending, &world, coord);
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
    runtime.pending.advance_cadence(
        world_ticks.ticks_this_frame(),
        game_rules.ticks_per_second(),
        &fluids,
    );

    let mut budget = FrameWorkBudget::new(
        FLUID_UPDATE_BUDGET,
        MIN_FLUID_UPDATES_BEFORE_BUDGET_CHECK,
    )
    .with_maximum_items(MAX_FLUID_UPDATES_PER_FRAME);

    classify_topology_updates(&mut runtime, &fluids, &mut budget);

    while !budget.exhausted() {
        let Some((scheduled_fluid_id, position)) = runtime.pending.pop_runnable_fluid() else {
            break;
        };
        budget.record(1);

        let Some((cell, current, _)) = runtime.world.sample_at(position) else {
            continue;
        };
        let desired = desired_fluid(&runtime.world, position, cell, current, &fluids);
        if current == desired {
            continue;
        }

        let Some(transition_fluid_id) = transition_fluid_id(current, desired) else {
            continue;
        };

        if transition_fluid_id != scheduled_fluid_id {
            runtime
                .pending
                .enqueue_fluid_priority(transition_fluid_id, position);
            continue;
        }

        if runtime.world.set_fluid_at(position, desired).is_none() {
            continue;
        }

        runtime.lighting.enqueue_medium_edit(position);
        enqueue_remesh(position, &mut runtime.remesh_queue);
        enqueue_changed_fluid_neighborhood(&mut runtime.pending, position, current, desired);
    }

    runtime.pending.discard_idle_step_credit();
}

fn classify_topology_updates(
    runtime: &mut FluidSimulationRuntime<'_>,
    fluids: &FluidRegistry,
    budget: &mut FrameWorkBudget,
) {
    let batch_len = runtime.pending.topology_queue.len();
    for _ in 0..batch_len {
        if budget.exhausted() {
            break;
        }

        let Some(position) = runtime.pending.pop_topology() else {
            break;
        };
        budget.record(1);

        let Some((cell, current, _)) = runtime.world.sample_at(position) else {
            continue;
        };
        let desired = desired_fluid(&runtime.world, position, cell, current, fluids);
        if current == desired {
            continue;
        }

        let Some(fluid_id) = transition_fluid_id(current, desired) else {
            continue;
        };
        runtime.pending.enqueue_fluid_priority(fluid_id, position);
    }
}

fn transition_fluid_id(
    current: Option<FluidCell>,
    desired: Option<FluidCell>,
) -> Option<FluidId> {
    desired
        .map(|fluid| fluid.fluid_id)
        .or_else(|| current.map(|fluid| fluid.fluid_id))
}

fn enqueue_changed_fluid_neighborhood(
    pending: &mut PendingFluidUpdates,
    position: IVec3,
    current: Option<FluidCell>,
    desired: Option<FluidCell>,
) {
    let current_id = current.map(|fluid| fluid.fluid_id);
    let desired_id = desired.map(|fluid| fluid.fluid_id);

    if let Some(fluid_id) = current_id {
        pending.enqueue_fluid_neighborhood(fluid_id, position);
    }
    if let Some(fluid_id) = desired_id
        && Some(fluid_id) != current_id
    {
        pending.enqueue_fluid_neighborhood(fluid_id, position);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluid_lane_freezes_newly_enqueued_work_until_the_next_step() {
        let mut pending = PendingFluidUpdates::default();
        let fluid_id = 0;

        pending.enqueue_fluid(fluid_id, IVec3::new(1, 2, 3));
        {
            let lane = pending.fluid_lane_mut(fluid_id);
            lane.ready_steps = 1;
        }

        assert_eq!(
            pending.pop_runnable_fluid(),
            Some((fluid_id, IVec3::new(1, 2, 3)))
        );

        pending.enqueue_fluid_neighborhood(fluid_id, IVec3::new(1, 2, 3));
        assert_eq!(pending.pop_runnable_fluid(), None);

        {
            let lane = pending.fluid_lane_mut(fluid_id);
            lane.ready_steps = 1;
        }
        assert!(pending.pop_runnable_fluid().is_some());
    }

    #[test]
    fn priority_work_does_not_enter_an_already_frozen_step() {
        let mut pending = PendingFluidUpdates::default();
        let fluid_id = 0;
        let first = IVec3::new(1, 2, 3);
        let second = IVec3::new(2, 2, 3);
        let urgent = IVec3::new(9, 2, 3);

        pending.enqueue_fluid(fluid_id, first);
        pending.enqueue_fluid(fluid_id, second);
        pending.fluid_lane_mut(fluid_id).ready_steps = 1;

        assert_eq!(pending.pop_runnable_fluid(), Some((fluid_id, first)));
        pending.enqueue_fluid_priority(fluid_id, urgent);
        assert_eq!(pending.pop_runnable_fluid(), Some((fluid_id, second)));
        assert_eq!(pending.pop_runnable_fluid(), None);

        pending.fluid_lane_mut(fluid_id).ready_steps = 1;
        assert_eq!(pending.pop_runnable_fluid(), Some((fluid_id, urgent)));
    }

    #[test]
    fn fluid_lane_keeps_unfinished_step_runnable_across_frames() {
        let mut pending = PendingFluidUpdates::default();
        let fluid_id = 0;

        pending.enqueue_fluid(fluid_id, IVec3::new(1, 2, 3));
        pending.enqueue_fluid(fluid_id, IVec3::new(2, 2, 3));
        {
            let lane = pending.fluid_lane_mut(fluid_id);
            lane.ready_steps = 1;
        }

        assert!(pending.pop_runnable_fluid().is_some());
        assert_eq!(pending.fluid_lanes[0].batch_remaining, 1);
        assert!(pending.pop_runnable_fluid().is_some());
        assert_eq!(pending.fluid_lanes[0].batch_remaining, 0);
    }

    #[test]
    fn transition_uses_destination_fluid_for_replacement() {
        let water = FluidCell::spreading(0, 4, 1);
        let lava = FluidCell::spreading(1, 8, 0);

        assert_eq!(transition_fluid_id(Some(water), Some(lava)), Some(1));
        assert_eq!(transition_fluid_id(Some(water), None), Some(0));
        assert_eq!(transition_fluid_id(None, Some(lava)), Some(1));
    }
}
