mod diagnostics;
mod frontier;
mod solver;
mod state;
mod settling;

use std::time::Duration;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::fluid::{FluidId, FluidRegistry},
    voxel::{
        coordinates::chunk_coord_from_world,
        fluid::FluidCell,
        lighting::PendingLightingUpdates,
        neighbors::HORIZONTAL_NEIGHBORS,
        world::VoxelWorld,
    },
};

use self::{
    diagnostics::FluidPerformanceDiagnostics,
    solver::{FluidSolverScratch, desired_fluid_with_scratch, enqueue_remesh},
    state::FluidTickKey,
};
pub(in crate::world) use self::settling::{
    GeneratedFluidSettling, GeneratedFluidSettlingCompletion,
};
pub(crate) use self::state::{PendingFluidUpdates, SavedFluidUpdates};
use super::{
    chunk_remesh::{ChunkRemeshQueue, prune_absent_remesh_halo},
    game_rules::GameRules,
    streaming::ChunkStreamingState,
    tick::WorldTickClock,
    work_budget::{FrameWorkBudget, WorldFrameWorkBudget},
};

const FLUID_UPDATE_BUDGET: Duration = Duration::from_millis(1);
const FLUID_CATCHUP_BUDGET: Duration = Duration::from_millis(3);
const MIN_FLUID_UPDATES_BEFORE_BUDGET_CHECK: usize = 64;
const MAX_FLUID_UPDATES_PER_FRAME: usize = 512;
const MAX_FLUID_CATCHUP_UPDATES_PER_FRAME: usize = 2_048;

#[derive(SystemParam)]
pub(super) struct FluidSimulationRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
    pending: ResMut<'w, PendingFluidUpdates>,
    lighting: ResMut<'w, PendingLightingUpdates>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
    streaming: Res<'w, ChunkStreamingState>,
    frame_budget: Res<'w, WorldFrameWorkBudget>,
}

pub(super) fn process_fluid_updates(
    world_ticks: Res<WorldTickClock>,
    game_rules: Res<GameRules>,
    fluids: Res<FluidRegistry>,
    time: Res<Time<Real>>,
    mut solver_scratch: Local<FluidSolverScratch>,
    mut diagnostics: Local<FluidPerformanceDiagnostics>,
    mut runtime: FluidSimulationRuntime,
) {
    let current_tick = world_ticks.current_tick();
    let ticks_per_second = game_rules.ticks_per_second();

    let catch_up = runtime.pending.should_catch_up();
    let mut budget = if catch_up {
        FrameWorkBudget::new(
            FLUID_CATCHUP_BUDGET,
            MIN_FLUID_UPDATES_BEFORE_BUDGET_CHECK,
        )
        .with_global_deadline(runtime.frame_budget.deadline())
        .with_maximum_items(MAX_FLUID_CATCHUP_UPDATES_PER_FRAME)
    } else {
        FrameWorkBudget::new(
            FLUID_UPDATE_BUDGET,
            MIN_FLUID_UPDATES_BEFORE_BUDGET_CHECK,
        )
        .with_global_deadline(runtime.frame_budget.deadline())
        .with_maximum_items(MAX_FLUID_UPDATES_PER_FRAME)
    };

    classify_topology_updates(
        &mut runtime,
        &fluids,
        &mut solver_scratch,
        current_tick,
        ticks_per_second,
        &mut budget,
    );

    process_due_fluid_ticks(
        &mut runtime,
        &fluids,
        &mut solver_scratch,
        current_tick,
        ticks_per_second,
        &mut budget,
    );

    schedule_frontier_wakes(
        &runtime.world,
        &mut runtime.pending,
        &fluids,
        current_tick,
        ticks_per_second,
        &mut budget,
    );

    diagnostics.record(
        time.delta(),
        solver_scratch.take_metrics(),
        &runtime.pending,
        catch_up,
    );
}

fn classify_topology_updates(
    runtime: &mut FluidSimulationRuntime<'_>,
    fluids: &FluidRegistry,
    solver_scratch: &mut FluidSolverScratch,
    current_tick: u64,
    ticks_per_second: u32,
    budget: &mut FrameWorkBudget,
) {
    let batch_len = runtime.pending.topology_len();
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
        let desired = desired_fluid_with_scratch(
            &runtime.world,
            position,
            cell,
            current,
            fluids,
            solver_scratch,
        );
        if current == desired {
            continue;
        }

        let Some(fluid_id) = transition_fluid_id(current, desired) else {
            continue;
        };
        schedule_fluid_tick_after_delay(
            &mut runtime.pending,
            fluids,
            fluid_id,
            position,
            current_tick,
            ticks_per_second,
        );
    }
}

fn process_due_fluid_ticks(
    runtime: &mut FluidSimulationRuntime<'_>,
    fluids: &FluidRegistry,
    solver_scratch: &mut FluidSolverScratch,
    current_tick: u64,
    ticks_per_second: u32,
    budget: &mut FrameWorkBudget,
) {
    while !budget.exhausted() {
        let Some((scheduled, _due_tick)) = runtime.pending.pop_due(current_tick) else {
            break;
        };
        budget.record(1);

        let position = scheduled.position;
        let coord = chunk_coord_from_world(position);
        if runtime.streaming.generated_chunk_is_unpublished(coord)
            || runtime
                .streaming
                .generated_fluid_settling_owns_mutation(coord)
        {
            // Generated chunks and any already-published chunks temporarily
            // owned by the settling closure are worldgen-owned. Reuse dormant
            // scheduling; publication/final reconciliation reactivates them.
            runtime.pending.defer_unloaded(scheduled);
            continue;
        }
        let Some((cell, current, _)) = runtime.world.sample_at(position) else {
            runtime.pending.defer_unloaded(scheduled);
            continue;
        };
        let desired = desired_fluid_with_scratch(
            &runtime.world,
            position,
            cell,
            current,
            fluids,
            solver_scratch,
        );
        if current == desired {
            continue;
        }

        let Some(transition_fluid_id) = transition_fluid_id(current, desired) else {
            continue;
        };

        if transition_fluid_id != scheduled.fluid_id {
            schedule_fluid_tick_after_delay(
                &mut runtime.pending,
                fluids,
                transition_fluid_id,
                position,
                current_tick,
                ticks_per_second,
            );
            continue;
        }

        if runtime.world.set_fluid_at(position, desired).is_none() {
            continue;
        }

        runtime.lighting.enqueue_medium_edit(position);
        enqueue_remesh(position, &mut runtime.remesh_queue);
        prune_absent_remesh_halo(position, &runtime.world, &mut runtime.remesh_queue);
        schedule_changed_fluid_neighborhood(
            &runtime.world,
            &mut runtime.pending,
            fluids,
            position,
            current,
            desired,
            current_tick,
            ticks_per_second,
        );
    }
}

fn schedule_frontier_wakes(
    world: &VoxelWorld,
    pending: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
    current_tick: u64,
    ticks_per_second: u32,
    budget: &mut FrameWorkBudget,
) {
    while !budget.exhausted() {
        let Some(wake) = pending.pop_wake() else {
            break;
        };
        budget.record(1);
        if !fluid_tick_target_can_change(world, wake.position) {
            continue;
        }
        schedule_fluid_tick_after_delay(
            pending,
            fluids,
            wake.fluid_id,
            wake.position,
            current_tick,
            ticks_per_second,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn schedule_changed_fluid_neighborhood(
    world: &VoxelWorld,
    pending: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
    position: IVec3,
    current: Option<FluidCell>,
    desired: Option<FluidCell>,
    current_tick: u64,
    ticks_per_second: u32,
) {
    let current_id = current.map(|fluid| fluid.fluid_id);
    let desired_id = desired.map(|fluid| fluid.fluid_id);

    if let Some(fluid_id) = current_id {
        schedule_fluid_neighborhood_after_delay(
            world,
            pending,
            fluids,
            fluid_id,
            position,
            current_tick,
            ticks_per_second,
        );
    }
    if let Some(fluid_id) = desired_id
        && Some(fluid_id) != current_id
    {
        schedule_fluid_neighborhood_after_delay(
            world,
            pending,
            fluids,
            fluid_id,
            position,
            current_tick,
            ticks_per_second,
        );
    }
}

fn schedule_fluid_neighborhood_after_delay(
    world: &VoxelWorld,
    pending: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
    fluid_id: FluidId,
    position: IVec3,
    current_tick: u64,
    ticks_per_second: u32,
) {
    let Some(delay) = fluid_tick_delay_for_id(fluids, fluid_id, ticks_per_second) else {
        return;
    };
    let due_tick = current_tick.saturating_add(delay);
    let targets = [
        position,
        position - IVec3::Y,
        position + HORIZONTAL_NEIGHBORS[0],
        position + HORIZONTAL_NEIGHBORS[1],
        position + HORIZONTAL_NEIGHBORS[2],
        position + HORIZONTAL_NEIGHBORS[3],
    ];
    let eligible = targets.map(|target| fluid_tick_target_can_change(world, target));

    if eligible.into_iter().all(|can_change| can_change) {
        pending.schedule_neighborhood(fluid_id, position, due_tick);
        return;
    }

    for (target, can_change) in targets.into_iter().zip(eligible) {
        if can_change {
            pending.schedule_at(
                FluidTickKey {
                    fluid_id,
                    position: target,
                },
                due_tick,
            );
        }
    }
}

fn fluid_tick_target_can_change(world: &VoxelWorld, position: IVec3) -> bool {
    if position.y < 0 {
        return false;
    }

    world.sample_at(position).is_none_or(|(cell, fluid, _)| {
        cell.is_none() && !fluid.is_some_and(FluidCell::is_source)
    })
}

fn schedule_fluid_tick_after_delay(
    pending: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
    fluid_id: FluidId,
    position: IVec3,
    current_tick: u64,
    ticks_per_second: u32,
) {
    let Some(delay) = fluid_tick_delay_for_id(fluids, fluid_id, ticks_per_second) else {
        return;
    };
    pending.schedule_at(
        FluidTickKey { fluid_id, position },
        current_tick.saturating_add(delay),
    );
}

fn fluid_tick_delay_for_id(
    fluids: &FluidRegistry,
    fluid_id: FluidId,
    ticks_per_second: u32,
) -> Option<u64> {
    let definition = fluids
        .get(fluid_id)
        .unwrap_or_else(|| panic!("missing fluid definition for id {fluid_id}"));
    fluid_tick_delay_ticks(definition.spread_speed, ticks_per_second)
}

fn fluid_tick_delay_ticks(spread_speed: f32, ticks_per_second: u32) -> Option<u64> {
    if spread_speed <= f32::EPSILON || ticks_per_second == 0 {
        return None;
    }

    let ticks = (ticks_per_second as f64 / f64::from(spread_speed))
        .round()
        .max(1.0);
    Some(ticks as u64)
}

fn transition_fluid_id(
    current: Option<FluidCell>,
    desired: Option<FluidCell>,
) -> Option<FluidId> {
    desired
        .map(|fluid| fluid.fluid_id)
        .or_else(|| current.map(|fluid| fluid.fluid_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{cell::VoxelCell, chunk::VoxelChunk};

    #[test]
    fn spread_speed_quantizes_to_world_tick_delay() {
        assert_eq!(fluid_tick_delay_ticks(16.0, 40), Some(3));
        assert_eq!(fluid_tick_delay_ticks(4.0, 40), Some(10));
        assert_eq!(fluid_tick_delay_ticks(80.0, 40), Some(1));
        assert_eq!(fluid_tick_delay_ticks(0.0, 40), None);
    }

    #[test]
    fn transition_uses_destination_fluid_for_replacement() {
        let water = FluidCell::spreading(0, 4, 1);
        let lava = FluidCell::spreading(1, 8, 0);

        assert_eq!(transition_fluid_id(Some(water), Some(lava)), Some(1));
        assert_eq!(transition_fluid_id(Some(water), None), Some(0));
        assert_eq!(transition_fluid_id(None, Some(lava)), Some(1));
    }

    #[test]
    fn fluid_tick_pruning_keeps_only_targets_that_can_change() {
        let mut world = VoxelWorld::default();
        world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());

        let empty = IVec3::new(1, 2, 1);
        let source = IVec3::new(2, 2, 1);
        let solid = IVec3::new(3, 2, 1);
        world.set_fluid_at(source, Some(FluidCell::source(0, 8)));
        world.set_block_at(
            solid,
            Some(VoxelCell::new("stone", Default::default())),
        );

        assert!(fluid_tick_target_can_change(&world, empty));
        assert!(!fluid_tick_target_can_change(&world, source));
        assert!(!fluid_tick_target_can_change(&world, solid));
        assert!(!fluid_tick_target_can_change(&world, IVec3::new(0, -1, 0)));

        // Keep unloaded targets schedulable so existing dormant-tick semantics
        // still carry edge propagation across streaming boundaries.
        assert!(fluid_tick_target_can_change(
            &world,
            IVec3::new(32, 2, 0),
        ));
    }
}