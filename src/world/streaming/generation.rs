use std::time::Duration;

use crate::world::{
    chunk_generation_tasks::MAX_GENERATION_TASKS_IN_FLIGHT,
    chunk_rendering::ChunkRenderPool,
    chunk_system_params::ChunkContent,
    generation_region::generation_region_coord,
    work_budget::FrameWorkBudget,
};

use super::{ChunkStreamingQueues, ChunkStreamingWork, seed_loaded_chunk_lighting};

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
        work.state.finish_generation_region();
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
            work.state.requeue(completed.coord);
            continue;
        }

        if let Some(region) = work.state.active_generation_region() {
            debug_assert_eq!(
                generation_region_coord(completed.coord),
                region,
                "completed streaming generation task must belong to the active region cohort"
            );
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

    let Some(region) = work.state.active_generation_region() else {
        return;
    };
    if work.generation_tasks.contains_generation_region(region)
        || work.state.generation_region_has_pending(region)
    {
        return;
    }

    let staged = work.state.take_staged_generated_chunks();
    if staged.is_empty() {
        work.state.finish_generation_region();
        return;
    }

    let world = &work.world;
    work.state.fluid_settling.begin(world, staged);

    if process_streaming_fluid_settling(content, work) {
        publish_settled_chunks(content, work, queues, current_tick);
        work.state.finish_generation_region();
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

    if work.state.fluid_settling.is_active() {
        return;
    }

    loop {
        if budget.exhausted() || work.generation_tasks.pending_count() >= max_in_flight {
            break;
        }

        let coord = if let Some(region) = work.state.active_generation_region() {
            work.state
                .pending
                .pop_where(|coord| generation_region_coord(coord) == region)
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
        if work.state.generated_chunk_is_unpublished(coord) {
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

        let region = work.state.begin_generation_region(coord);
        debug_assert_eq!(generation_region_coord(coord), region);

        if work.generation_tasks.schedule(coord) {
            budget.record(1);
        } else {
            work.state.defer_pending(coord);
            budget.record(1);
        }
    }
}
