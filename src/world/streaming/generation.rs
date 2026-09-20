use std::time::Duration;

use crate::world::{
    chunk_generation_tasks::MAX_GENERATION_TASKS_IN_FLIGHT,
    chunk_rendering::ChunkRenderPool,
    chunk_system_params::ChunkContent,
    work_budget::FrameWorkBudget,
};

use super::{
    ChunkStreamingQueues, ChunkStreamingWork, is_critical_streaming_coord,
    seed_loaded_chunk_lighting,
};

const MAX_GENERATION_DISPATCH_WORK_PER_FRAME: usize = 16;
const MAX_GENERATION_TASKS_WITH_MESH_BACKLOG: usize = 4;
const MAX_GENERATION_RESULTS_COLLECTED_PER_FRAME: usize = 8;
const GENERATION_DISPATCH_BUDGET: Duration = Duration::from_millis(1);
const GENERATION_RESULT_INTEGRATION_BUDGET: Duration = Duration::from_millis(1);
const STREAMING_FLUID_SETTLING_BUDGET: Duration = Duration::from_millis(1);
const MIN_STREAMING_FLUID_SETTLING_UPDATES: usize = 8;
const MAX_STREAMING_FLUID_SETTLING_UPDATES: usize = 256;

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
        publish_settled_chunks(content, work, queues, current_tick);
    }

    let current_revision = work.generation_tasks.revision();
    let mut budget = FrameWorkBudget::new(GENERATION_RESULT_INTEGRATION_BUDGET, 1)
        .with_maximum_items(MAX_GENERATION_RESULTS_COLLECTED_PER_FRAME);
    let mut generated_coords = Vec::new();

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = work.generation_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        if completed.revision != current_revision {
            work.state.requeue(completed.coord);
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
        generated_coords.push(completed.coord);
    }

    if generated_coords.is_empty() {
        return;
    }

    let world = &work.world;
    work.state
        .fluid_settling
        .begin(world, generated_coords);

    if process_streaming_fluid_settling(content, work) {
        publish_settled_chunks(content, work, queues, current_tick);
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

fn publish_settled_chunks(
    content: &ChunkContent<'_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
    let completed = work
        .state
        .fluid_settling
        .take_completed_chunks()
        .expect("completed streaming fluid settling must own generated chunks");
    for coord in completed {
        if !work.state.keeps_loaded(coord) {
            work.world.archive_chunk(coord);
            continue;
        }
        seed_loaded_chunk_lighting(coord, content, work, queues, current_tick);
        work.state.mark_ready(coord);
    }
}

pub(super) fn dispatch_generation_tasks(
    content: &ChunkContent<'_>,
    render_pool: &ChunkRenderPool,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
    let max_in_flight = if work.mesh_tasks.pending_count() > 0 {
        MAX_GENERATION_TASKS_WITH_MESH_BACKLOG
    } else {
        MAX_GENERATION_TASKS_IN_FLIGHT
    };
    let mut budget = FrameWorkBudget::new(GENERATION_DISPATCH_BUDGET, 1)
        .with_maximum_items(MAX_GENERATION_DISPATCH_WORK_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let at_capacity = work.generation_tasks.pending_count() >= max_in_flight;
        let coord = if at_capacity {
            work.state.pop_critical_pending()
        } else {
            work.state.pending.pop()
        };
        let Some(coord) = coord else {
            break;
        };

        if render_pool.contains(coord)
            || work.state.ready.contains(coord)
            || work.mesh_tasks.contains(coord)
        {
            continue;
        }
        if work.generation_tasks.contains(coord) {
            continue;
        }
        if work.state.generated_chunk_is_settling(coord) {
            // Selection can change while a generated chunk is resident but
            // still unpublished. Settling, not the generic resident path,
            // owns its transition to ready.
            budget.record(1);
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

        if work.generation_tasks.pending_count() >= max_in_flight {
            let Some(center) = work.state.center else {
                work.state.requeue(coord);
                break;
            };
            let Some(preempted) = work
                .generation_tasks
                .cancel_farthest_where(center, |task_coord| {
                    !is_critical_streaming_coord(task_coord, center)
                })
            else {
                work.state.requeue(coord);
                break;
            };
            work.state.defer_pending(preempted);
        }

        if work.generation_tasks.schedule(coord) {
            budget.record(1);
        } else {
            work.state.defer_pending(coord);
            budget.record(1);
        }
    }
}
