use crate::world::{
    PendingFluidUpdates, WorldLoadMode,
    chunk_generation_tasks::{ChunkGenerationTasks, MAX_GENERATION_TASKS_IN_FLIGHT},
    chunk_system_params::{ChunkContent, ChunkGeneration},
    work_budget::FrameWorkBudget,
};

use super::{INITIAL_LOADING_BUDGET, super::{WorldLoadingPhase, system_params::WorldSetupProgress}};

pub(super) fn generate_initial_chunks(
    generation: &ChunkGeneration<'_>,
    content: &ChunkContent<'_>,
    progress: &mut WorldSetupProgress<'_>,
    fluid_updates: &mut PendingFluidUpdates,
    generation_tasks: &mut ChunkGenerationTasks,
    load_mode: WorldLoadMode,
) {
    generation_tasks.sync_snapshot(generation, content);
    let mut budget = FrameWorkBudget::new(INITIAL_LOADING_BUDGET, 1);

    integrate_generated_chunks(
        &mut budget,
        progress,
        fluid_updates,
        generation_tasks,
        load_mode,
    );
    dispatch_generation_tasks(
        &mut budget,
        progress,
        fluid_updates,
        generation_tasks,
        load_mode,
    );

    if progress.loading_state.generation_cursor >= progress.loading_state.coords.len()
        && progress.loading_state.generated >= progress.loading_state.coords.len()
        && generation_tasks.pending_count() == 0
    {
        progress.loading_state.phase = match load_mode {
            WorldLoadMode::New => WorldLoadingPhase::PrimingFluids,
            WorldLoadMode::Load => WorldLoadingPhase::Lighting,
        };
    }
}

fn integrate_generated_chunks(
    budget: &mut FrameWorkBudget,
    progress: &mut WorldSetupProgress<'_>,
    fluid_updates: &mut PendingFluidUpdates,
    generation_tasks: &mut ChunkGenerationTasks,
    load_mode: WorldLoadMode,
) {
    let current_revision = generation_tasks.revision();

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = generation_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        if completed.revision != current_revision {
            assert!(
                generation_tasks.schedule(completed.coord),
                "stale bootstrap generation must be rescheduled for {:?}",
                completed.coord
            );
            continue;
        }

        if progress.world.chunk(completed.coord).is_none() {
            progress
                .world
                .insert_chunk(completed.coord, completed.output);
        }
        if load_mode == WorldLoadMode::Load {
            fluid_updates.enqueue_loaded_fluid_frontier(&progress.world, completed.coord);
        }
        progress.loading_state.generated += 1;
    }
}

fn dispatch_generation_tasks(
    budget: &mut FrameWorkBudget,
    progress: &mut WorldSetupProgress<'_>,
    fluid_updates: &mut PendingFluidUpdates,
    generation_tasks: &mut ChunkGenerationTasks,
    load_mode: WorldLoadMode,
) {
    while generation_tasks.pending_count() < MAX_GENERATION_TASKS_IN_FLIGHT {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = progress
            .loading_state
            .coords
            .get(progress.loading_state.generation_cursor)
            .copied()
        else {
            break;
        };

        if progress.world.chunk(coord).is_some() {
            if load_mode == WorldLoadMode::Load {
                fluid_updates.enqueue_loaded_fluid_frontier(&progress.world, coord);
            }
            progress.loading_state.generation_cursor += 1;
            progress.loading_state.generated += 1;
            budget.record(1);
            continue;
        }

        if progress.world.has_resident_or_persisted_chunk(coord) {
            assert!(
                progress.world.restore_chunk(coord),
                "generated bootstrap chunk must be resident or archived: {coord:?}"
            );
            if load_mode == WorldLoadMode::Load {
                fluid_updates.enqueue_loaded_fluid_frontier(&progress.world, coord);
            }
            progress.loading_state.generation_cursor += 1;
            progress.loading_state.generated += 1;
            budget.record(1);
            continue;
        }

        if !generation_tasks.schedule(coord) {
            break;
        }
        progress.loading_state.generation_cursor += 1;
    }
}
