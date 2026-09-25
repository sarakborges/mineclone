use std::time::Duration;

use bevy::{
    platform::collections::HashSet,
    prelude::{IVec2, IVec3},
};

use crate::{
    voxel::{
        chunk::VoxelChunk,
        coordinates::{
            chunk_coord_from_world, visit_chunk_coords_whose_voxel_halo_contains,
        },
    },
    world::{
        chunk_generation_tasks::MAX_GENERATION_TASKS_IN_FLIGHT,
        chunk_rendering::ChunkRenderPool,
        chunk_system_params::ChunkContent,
        fluid_updates::GeneratedFluidSettlingCompletion,
        work_budget::FrameWorkBudget,
    },
};

use super::{ChunkStreamingQueues, ChunkStreamingWork, seed_loaded_chunk_lighting};

const MAX_GENERATION_DISPATCH_WORK_PER_FRAME: usize = 16;
const MAX_CRITICAL_GENERATION_WAVE_TARGETS: usize = 4;
const MAX_GENERATION_RESULTS_COLLECTED_PER_FRAME: usize = 8;
const GENERATION_DISPATCH_BUDGET: Duration = Duration::from_millis(1);
const GENERATION_RESULT_INTEGRATION_BUDGET: Duration = Duration::from_millis(1);
const STREAMING_FLUID_SETTLING_BUDGET: Duration = Duration::from_millis(1);
const MIN_STREAMING_FLUID_SETTLING_UPDATES: usize = 1;
const MAX_STREAMING_FLUID_SETTLING_UPDATES: usize = 128;

pub(super) fn collect_generated_chunks(
    content: &ChunkContent<'_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
    if work.state.fluid_settling.is_active() {
        if !process_streaming_fluid_settling(content, work) {
            return;
        }
        if extend_generation_wave_for_fluid_closure(
            content,
            work,
            queues,
            current_tick,
        ) {
            return;
        }
        let completion = work
            .state
            .fluid_settling
            .take_completion()
            .expect("completed streaming fluid settling must own its generation wave");
        let completion = work.state.merge_settling_rounds(completion);
        publish_settled_wave(completion, content, work, queues, current_tick);
        work.state.finish_generation_wave();
        return;
    }

    let deadline = work.frame_budget.deadline();
    let current_revision = work.generation_tasks.revision();
    let mut budget = FrameWorkBudget::new(GENERATION_RESULT_INTEGRATION_BUDGET, 1)
        .with_global_deadline(deadline)
        .with_maximum_items(MAX_GENERATION_RESULTS_COLLECTED_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = work.generation_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        let horizontal = IVec2::new(completed.coord.x, completed.coord.z);
        if let Some(structure_top_chunk) = work
            .generation_tasks
            .structure_top_chunk_if_ready(horizontal)
        {
            work.state
                .adopt_structure_top_chunk(horizontal, structure_top_chunk);
        }

        if completed.revision != current_revision {
            if work.state.keeps_loaded(completed.coord)
                && work.state.generation_wave_targets.contains(&completed.coord)
            {
                work.state.generation_wave_pending.enqueue(completed.coord);
            } else {
                work.state.abandon_generation_wave_target(completed.coord);
            }
            continue;
        }
        if !work.state.keeps_loaded(completed.coord) {
            work.state.abandon_generation_wave_target(completed.coord);
            continue;
        }
        if work.world.has_resident_or_persisted_chunk(completed.coord) {
            if work.world.chunk(completed.coord).is_none() {
                assert!(
                    work.world.restore_chunk(completed.coord),
                    "resident or persisted chunk must remain resident or archived: {:?}",
                    completed.coord
                );
            }
            seed_loaded_chunk_lighting(completed.coord, content, work, queues, current_tick);
            work.state.mark_ready(completed.coord);
            work.state.complete_generation_wave_target(completed.coord);
            continue;
        }

        let requires_fluid_settling = generated_chunk_requires_fluid_settling(
            &work.world,
            completed.coord,
            &completed.output,
        );
        work.world.insert_chunk(completed.coord, completed.output);
        if requires_fluid_settling {
            work.state.stage_generated_chunk(completed.coord);
        } else {
            seed_loaded_chunk_lighting(
                completed.coord,
                content,
                work,
                queues,
                current_tick,
            );
            work.state.mark_ready(completed.coord);
            work.state.complete_generation_wave_target(completed.coord);
        }
    }

    if work.generation_tasks.pending_count() > 0
        || work.state.generation_wave_pending.len() > 0
    {
        return;
    }

    let staged = work.state.take_staged_generated_chunks();
    if staged.is_empty() {
        work.state.finish_generation_wave();
        return;
    }

    let world = &work.world;
    work.state
        .fluid_settling
        .begin(world, content.fluids(), staged);

    if process_streaming_fluid_settling(content, work) {
        if extend_generation_wave_for_fluid_closure(
            content,
            work,
            queues,
            current_tick,
        ) {
            return;
        }
        let completion = work
            .state
            .fluid_settling
            .take_completion()
            .expect("completed streaming fluid settling must own its generation wave");
        let completion = work.state.merge_settling_rounds(completion);
        publish_settled_wave(completion, content, work, queues, current_tick);
        work.state.finish_generation_wave();
    }
}

fn extend_generation_wave_for_fluid_closure(
    content: &ChunkContent<'_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) -> bool {
    let required = work
        .state
        .fluid_settling
        .required_unloaded_frontier_chunks(&work.world)
        .into_iter()
        .filter(|coord| work.state.keeps_loaded(*coord))
        .filter(|coord| work.world.chunk(*coord).is_none())
        .collect::<Vec<_>>();

    if required.is_empty() {
        return false;
    }

    let completion = work
        .state
        .fluid_settling
        .take_completion()
        .expect("fluid closure extension requires completed settling");
    work.state.retain_settling_round(completion);

    for coord in required {
        if work.world.has_resident_or_persisted_chunk(coord) {
            work.state.remove_pending(coord);
            assert!(
                work.world.restore_chunk(coord),
                "fluid dependency chunk must restore from archived state: {coord:?}"
            );
            seed_loaded_chunk_lighting(coord, content, work, queues, current_tick);
            work.state.mark_ready(coord);
            continue;
        }

        work.state.adopt_generation_wave_target(coord);
    }

    true
}

fn process_streaming_fluid_settling(
    content: &ChunkContent<'_>,
    work: &mut ChunkStreamingWork<'_>,
) -> bool {
    let deadline = work.frame_budget.deadline();
    let mut settling_budget = FrameWorkBudget::new(
        STREAMING_FLUID_SETTLING_BUDGET,
        MIN_STREAMING_FLUID_SETTLING_UPDATES,
    )
    .with_global_deadline(deadline)
    .with_maximum_items(MAX_STREAMING_FLUID_SETTLING_UPDATES);

    let state = &mut work.state;
    let world = &mut work.world;
    state
        .fluid_settling
        .process(world, content.fluids(), &mut settling_budget)
}

fn publish_settled_wave(
    completion: GeneratedFluidSettlingCompletion,
    content: &ChunkContent<'_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
    reconcile_existing_fluid_changes(
        &completion.changed_existing_positions,
        work,
        queues,
    );

    for &coord in &completion.owned_existing_chunks {
        queues.fluid.reactivate_loaded_chunk(coord, current_tick);
        queues.fluid.enqueue_loaded_fluid_frontier(&work.world, coord);
    }

    for coord in completion.generated_chunks {
        if !work.state.keeps_loaded(coord) {
            work.world.archive_chunk(coord);
            work.state.complete_generation_wave_target(coord);
            continue;
        }
        seed_loaded_chunk_lighting(coord, content, work, queues, current_tick);
        work.state.mark_ready(coord);
        work.state.complete_generation_wave_target(coord);
    }
}

fn reconcile_existing_fluid_changes(
    changed_positions: &[IVec3],
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
) {
    if changed_positions.is_empty() {
        return;
    }

    queues
        .lighting
        .enqueue_settling_medium_edits(changed_positions.iter().copied());

    let mut changed_chunks = HashSet::new();
    let mut affected_fluid_meshes = HashSet::new();
    for &position in changed_positions {
        let coord = chunk_coord_from_world(position);
        changed_chunks.insert(coord);
        visit_chunk_coords_whose_voxel_halo_contains(position, |affected| {
            affected_fluid_meshes.insert(affected);
        });
    }

    for affected in affected_fluid_meshes {
        queues
            .lighting
            .defer_settling_fluid_remesh(affected, changed_chunks.contains(&affected));
    }

    let mut changed_chunks = changed_chunks.into_iter().collect::<Vec<_>>();
    changed_chunks.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
    for coord in changed_chunks {
        queues.fluid.enqueue_loaded_fluid_frontier(&work.world, coord);
    }
}

pub(super) fn dispatch_generation_tasks(
    content: &ChunkContent<'_>,
    render_pool: &ChunkRenderPool,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
    if work.state.fluid_settling.is_active() {
        return;
    }

    let deadline = work.frame_budget.deadline();
    let mut budget = FrameWorkBudget::new(GENERATION_DISPATCH_BUDGET, 1)
        .with_global_deadline(deadline)
        .with_maximum_items(MAX_GENERATION_DISPATCH_WORK_PER_FRAME);

    if work.state.generation_wave_accepts_new_targets() {
        select_generation_wave(
            content,
            render_pool,
            work,
            queues,
            current_tick,
            &mut budget,
        );
    }

    let attempts = work.state.generation_wave_pending.len();
    for _ in 0..attempts {
        if budget.exhausted()
            || work.generation_tasks.pending_count() >= MAX_GENERATION_TASKS_IN_FLIGHT
        {
            break;
        }

        let Some(coord) = work.state.generation_wave_pending.pop() else {
            break;
        };
        if !work.state.keeps_loaded(coord) {
            work.state.abandon_generation_wave_target(coord);
            budget.record(1);
            continue;
        }
        if work.generation_tasks.contains(coord) {
            budget.record(1);
            continue;
        }

        if work.generation_tasks.schedule(coord, &work.async_work) {
            budget.record(1);
        } else {
            work.state.generation_wave_pending.enqueue(coord);
            budget.record(1);
        }
    }
}

fn generated_chunk_requires_fluid_settling(
    world: &crate::voxel::world::VoxelWorld,
    coord: IVec3,
    chunk: &VoxelChunk,
) -> bool {
    if chunk.has_fluid_settling_work() {
        return true;
    }

    for y in -1..=1 {
        for z in -1..=1 {
            for x in -1..=1 {
                let offset = IVec3::new(x, y, z);
                if offset == IVec3::ZERO {
                    continue;
                }
                if world
                    .chunk(coord + offset)
                    .is_some_and(VoxelChunk::has_fluid_settling_work)
                {
                    return true;
                }
            }
        }
    }

    false
}

fn generation_wave_target_limit(state: &super::ChunkStreamingState) -> usize {
    let Some(center) = state.center else {
        return MAX_GENERATION_TASKS_IN_FLIGHT;
    };

    let has_critical_pending = state
        .pending
        .values()
        .any(|coord| super::is_critical_streaming_coord(coord, center));

    if has_critical_pending {
        MAX_CRITICAL_GENERATION_WAVE_TARGETS.min(MAX_GENERATION_TASKS_IN_FLIGHT)
    } else {
        MAX_GENERATION_TASKS_IN_FLIGHT
    }
}

fn select_generation_wave(
    content: &ChunkContent<'_>,
    render_pool: &ChunkRenderPool,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
    budget: &mut FrameWorkBudget,
) {
    let target_limit = generation_wave_target_limit(&work.state);
    while work.state.generation_wave_targets.len() < target_limit {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = work.state.pop_pending_by_priority() else {
            break;
        };

        if render_pool.contains(coord)
            || work.state.ready.contains(coord)
            || work.mesh_tasks.contains(coord)
            || work.generation_tasks.contains(coord)
            || work.state.generated_chunk_is_unpublished(coord)
        {
            continue;
        }

        if work.world.has_resident_or_persisted_chunk(coord) {
            assert!(
                work.world.restore_chunk(coord),
                "resident or persisted chunk must remain resident or archived: {coord:?}"
            );
            seed_loaded_chunk_lighting(coord, content, work, queues, current_tick);
            work.state.mark_ready(coord);
            budget.record(1);
            continue;
        }

        work.state.start_generation_wave_target(coord);
        budget.record(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::fluid::FluidCell;

    #[test]
    fn dry_generated_chunks_skip_settling_only_without_fluid_neighbors() {
        let coord = IVec3::new(3, 2, -4);
        let dry = VoxelChunk::empty();
        let mut world = crate::voxel::world::VoxelWorld::default();

        assert!(!generated_chunk_requires_fluid_settling(&world, coord, &dry));

        let mut static_water = VoxelChunk::empty();
        static_water.edit_initial_fluids(|fluids| {
            fluids.set_fluid(4, 5, 6, FluidCell::source(0, 8));
        });
        static_water.suppress_generated_fluid_frontiers(&[[4, 5, 6]]);
        assert!(!generated_chunk_requires_fluid_settling(
            &world,
            coord,
            &static_water
        ));

        let mut flowing = VoxelChunk::empty();
        flowing.set_fluid(4, 5, 6, Some(FluidCell::spreading(0, 7, 1)));
        assert!(generated_chunk_requires_fluid_settling(&world, coord, &flowing));

        world.insert_chunk(coord + IVec3::X, flowing);
        assert!(generated_chunk_requires_fluid_settling(&world, coord, &dry));
    }

    #[test]
    fn critical_generation_frontier_uses_smaller_publication_waves() {
        let center = IVec3::new(10, 2, -4);
        let mut state = super::super::ChunkStreamingState {
            center: Some(center),
            ..Default::default()
        };
        state.pending.enqueue(center + IVec3::X);

        assert_eq!(
            generation_wave_target_limit(&state),
            MAX_CRITICAL_GENERATION_WAVE_TARGETS,
        );

        state.pending.clear();
        state.pending.enqueue(center + IVec3::new(4, 0, 0));
        assert_eq!(
            generation_wave_target_limit(&state),
            MAX_GENERATION_TASKS_IN_FLIGHT,
        );
    }
}
