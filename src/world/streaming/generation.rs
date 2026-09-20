use std::time::Duration;

use bevy::{
    platform::collections::HashSet,
    prelude::IVec3,
};

use crate::{
    voxel::{
        coordinates::chunk_coord_from_world,
        neighbors::CARDINAL_NEIGHBORS,
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
        let completion = work
            .state
            .fluid_settling
            .take_completion()
            .expect("completed streaming fluid settling must own its generation wave");
        publish_settled_wave(completion, content, work, queues, current_tick);
        work.state.finish_generation_wave();
        return;
    }

    let current_revision = work.generation_tasks.revision();
    let mut budget = FrameWorkBudget::new(GENERATION_RESULT_INTEGRATION_BUDGET, 1)
        .with_maximum_items(MAX_GENERATION_RESULTS_COLLECTED_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = work.generation_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        if completed.revision != current_revision {
            if work.state.keeps_loaded(completed.coord)
                && work.state.generation_wave_targets.contains(&completed.coord)
            {
                work.state.generation_wave_pending.enqueue(completed.coord);
            }
            continue;
        }
        if !work.state.keeps_loaded(completed.coord) {
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
            continue;
        }

        work.world.insert_chunk(completed.coord, completed.output);
        work.state.stage_generated_chunk(completed.coord);
    }

    if work.generation_tasks.pending_count() > 0
        || work.state.generation_wave_pending.len() > 0
    {
        return;
    }

    let staged = work.state.take_staged_generated_chunks();
    if staged.is_empty() {
        if !work.state.generation_wave_targets.is_empty() {
            work.state.finish_generation_wave();
        }
        return;
    }

    let world = &work.world;
    work.state
        .fluid_settling
        .begin(world, content.fluids(), staged);

    if process_streaming_fluid_settling(content, work) {
        let completion = work
            .state
            .fluid_settling
            .take_completion()
            .expect("completed streaming fluid settling must own its generation wave");
        publish_settled_wave(completion, content, work, queues, current_tick);
        work.state.finish_generation_wave();
    }
}

fn process_streaming_fluid_settling(
    content: &ChunkContent<'_>,
    work: &mut ChunkStreamingWork<'_>,
) -> bool {
    let mut settling_budget = FrameWorkBudget::new(
        STREAMING_FLUID_SETTLING_BUDGET,
        MIN_STREAMING_FLUID_SETTLING_UPDATES,
    )
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

    for coord in completion.generated_chunks {
        if !work.state.keeps_loaded(coord) {
            work.world.archive_chunk(coord);
            continue;
        }
        seed_loaded_chunk_lighting(coord, content, work, queues, current_tick);
        work.state.mark_ready(coord);
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

    let mut changed_chunks = HashSet::new();
    for &position in changed_positions {
        queues.lighting.enqueue_medium_edit(position);
        let coord = chunk_coord_from_world(position);
        changed_chunks.insert(coord);
        queues.remesh.enqueue_fluid_priority(coord);
        for offset in CARDINAL_NEIGHBORS {
            let neighbor = coord + offset;
            if neighbor.y >= 0 {
                queues.remesh.enqueue_fluid(neighbor);
            }
        }
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

    let mut budget = FrameWorkBudget::new(GENERATION_DISPATCH_BUDGET, 1)
        .with_maximum_items(MAX_GENERATION_DISPATCH_WORK_PER_FRAME);

    if work.state.generation_wave_targets.is_empty() {
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
            budget.record(1);
            continue;
        }
        if work.generation_tasks.contains(coord) {
            budget.record(1);
            continue;
        }

        if work.generation_tasks.schedule(coord) {
            budget.record(1);
        } else {
            work.state.generation_wave_pending.enqueue(coord);
            budget.record(1);
        }
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
    while work.state.generation_wave_targets.len() < MAX_GENERATION_TASKS_IN_FLIGHT {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = work.state.pending.pop() else {
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
