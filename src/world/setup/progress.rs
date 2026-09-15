use std::time::Duration;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::{
        player_id::LOCAL_PLAYER_ID, player_position_is_clear, safe_spawn_position,
        spawn_player_entity,
    },
    ui::transition::ScreenTransitionTarget,
    voxel::{lighting::initialize_chunks_lighting, mesh_snapshot::ChunkMeshSnapshot},
};

use super::{
    WorldLoadingPhase,
    system_params::{WorldSetupPersistence, WorldSetupRuntime},
};
use crate::world::{
    WorldLoadMode,
    chunk_generation_tasks::MAX_GENERATION_TASKS_IN_FLIGHT,
    chunk_mesh_tasks::MAX_MESH_TASKS_IN_FLIGHT,
    chunk_rendering::spawn_built_chunk_meshes,
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    work_budget::FrameWorkBudget,
};

const BOOTSTRAP_LIGHT_BATCH_CHUNKS: usize = 2;
const INITIAL_LOADING_BUDGET: Duration = Duration::from_millis(12);

pub(in crate::world) fn setup_world(
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    mut runtime: WorldSetupRuntime,
    persistence: WorldSetupPersistence,
) {
    if runtime.transition.is_active() {
        return;
    }

    if !runtime.loading_state.screen_rendered {
        runtime.loading_state.screen_rendered = true;
        return;
    }

    match runtime.loading_state.phase {
        WorldLoadingPhase::Generating => {
            generate_initial_chunks(&generation, &content, &mut runtime)
        }
        WorldLoadingPhase::Lighting => light_initial_chunks(&content, &mut runtime),
        WorldLoadingPhase::Meshing => mesh_initial_chunks(&content, &mut renderer, &mut runtime),
        WorldLoadingPhase::Spawning => {
            spawn_loaded_world(&mut renderer, &mut runtime, &persistence)
        }
    }
}

fn generate_initial_chunks(
    generation: &ChunkGeneration<'_>,
    content: &ChunkContent<'_>,
    runtime: &mut WorldSetupRuntime<'_>,
) {
    runtime
        .generation_tasks
        .sync_snapshot(generation, content);
    let mut budget = FrameWorkBudget::new(INITIAL_LOADING_BUDGET, 1);

    integrate_generated_chunks(&mut budget, runtime);
    dispatch_generation_tasks(&mut budget, runtime);

    if runtime.loading_state.generation_cursor >= runtime.loading_state.coords.len()
        && runtime.loading_state.generated >= runtime.loading_state.coords.len()
        && runtime.generation_tasks.pending_count() == 0
    {
        runtime.loading_state.phase = WorldLoadingPhase::Lighting;
    }
}

fn integrate_generated_chunks(
    budget: &mut FrameWorkBudget,
    runtime: &mut WorldSetupRuntime<'_>,
) {
    let current_revision = runtime.generation_tasks.revision();

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = runtime.generation_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        if completed.revision != current_revision {
            assert!(
                runtime.generation_tasks.schedule(completed.coord),
                "stale bootstrap generation must be rescheduled for {:?}",
                completed.coord
            );
            continue;
        }

        if runtime.world.chunk(completed.coord).is_none() {
            runtime.world.insert_chunk(completed.coord, completed.output);
        }
        runtime
            .fluid_updates
            .enqueue_loaded_fluid_frontier(&runtime.world, completed.coord);
        runtime.loading_state.generated += 1;
    }
}

fn dispatch_generation_tasks(
    budget: &mut FrameWorkBudget,
    runtime: &mut WorldSetupRuntime<'_>,
) {
    while runtime.generation_tasks.pending_count() < MAX_GENERATION_TASKS_IN_FLIGHT {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = runtime
            .loading_state
            .coords
            .get(runtime.loading_state.generation_cursor)
            .copied()
        else {
            break;
        };

        if runtime.world.chunk(coord).is_some() {
            runtime
                .fluid_updates
                .enqueue_loaded_fluid_frontier(&runtime.world, coord);
            runtime.loading_state.generation_cursor += 1;
            runtime.loading_state.generated += 1;
            budget.record(1);
            continue;
        }

        if runtime.world.has_generated_chunk(coord) {
            assert!(
                runtime.world.restore_chunk(coord),
                "generated bootstrap chunk must be resident or archived: {coord:?}"
            );
            runtime
                .fluid_updates
                .enqueue_loaded_fluid_frontier(&runtime.world, coord);
            runtime.loading_state.generation_cursor += 1;
            runtime.loading_state.generated += 1;
            budget.record(1);
            continue;
        }

        if !runtime.generation_tasks.schedule(coord) {
            break;
        }
        runtime.loading_state.generation_cursor += 1;
    }
}

fn light_initial_chunks(content: &ChunkContent<'_>, runtime: &mut WorldSetupRuntime<'_>) {
    let mut budget = FrameWorkBudget::new(INITIAL_LOADING_BUDGET, 1);

    loop {
        if budget.exhausted() {
            break;
        }

        let start = runtime.loading_state.lit;
        if start >= runtime.loading_state.coords.len() {
            break;
        }
        let end = (start + BOOTSTRAP_LIGHT_BATCH_CHUNKS).min(runtime.loading_state.coords.len());

        drop(initialize_chunks_lighting(
            &mut runtime.world,
            &runtime.loading_state.coords[start..end],
            content.blocks(),
            content.fluids(),
            content.secondary_properties(),
        ));
        runtime.loading_state.lit = end;
        budget.record(end - start);
    }

    if runtime.loading_state.lit >= runtime.loading_state.coords.len() {
        runtime.loading_state.phase = WorldLoadingPhase::Meshing;
    }
}

fn mesh_initial_chunks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    runtime: &mut WorldSetupRuntime<'_>,
) {
    runtime.mesh_tasks.sync_snapshot(content);
    let mut budget = FrameWorkBudget::new(INITIAL_LOADING_BUDGET, 1);

    integrate_built_chunk_meshes(content, renderer, &mut budget, runtime);
    dispatch_mesh_tasks(&mut budget, runtime);

    if runtime.loading_state.mesh_cursor >= runtime.loading_state.coords.len()
        && runtime.loading_state.meshed >= runtime.loading_state.coords.len()
        && runtime.mesh_tasks.pending_count() == 0
    {
        runtime.loading_state.phase = WorldLoadingPhase::Spawning;
    }
}

fn integrate_built_chunk_meshes(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    budget: &mut FrameWorkBudget,
    runtime: &mut WorldSetupRuntime<'_>,
) {
    let current_revision = runtime.mesh_tasks.revision();

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = runtime.mesh_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        if completed.revision != current_revision {
            let snapshot = ChunkMeshSnapshot::capture(&runtime.world, completed.coord)
                .unwrap_or_else(|| panic!("generated chunk data should exist at {:?}", completed.coord));
            assert!(
                runtime.mesh_tasks.schedule(completed.coord, snapshot),
                "stale bootstrap mesh must be rescheduled for {:?}",
                completed.coord
            );
            continue;
        }

        let render_context = content.render_context(
            &runtime.world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );
        spawn_built_chunk_meshes(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            completed.coord,
            completed.output,
            &render_context,
        );
        runtime.loading_state.meshed += 1;
    }
}

fn dispatch_mesh_tasks(
    budget: &mut FrameWorkBudget,
    runtime: &mut WorldSetupRuntime<'_>,
) {
    while runtime.mesh_tasks.pending_count() < MAX_MESH_TASKS_IN_FLIGHT {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = runtime
            .loading_state
            .coords
            .get(runtime.loading_state.mesh_cursor)
            .copied()
        else {
            break;
        };

        let snapshot = ChunkMeshSnapshot::capture(&runtime.world, coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        if !runtime.mesh_tasks.schedule(coord, snapshot) {
            break;
        }

        runtime.loading_state.mesh_cursor += 1;
        budget.record(1);
    }
}

fn spawn_loaded_world(
    renderer: &mut ChunkRenderer<'_, '_>,
    runtime: &mut WorldSetupRuntime<'_>,
    persistence: &WorldSetupPersistence<'_>,
) {
    if runtime.loading_state.transition_requested {
        return;
    }

    let saved_position = (*persistence.load_mode == WorldLoadMode::Load)
        .then(|| persistence.save.player_position(LOCAL_PLAYER_ID))
        .flatten();
    let translation = saved_position
        .filter(|position| player_position_is_clear(&runtime.world, *position))
        .unwrap_or_else(|| safe_spawn_position(&runtime.world, runtime.loading_state.spawn_column));
    let game_mode = if *persistence.load_mode == WorldLoadMode::Load {
        persistence.save.player_game_mode(LOCAL_PLAYER_ID)
    } else {
        persistence.new_world_config.game_mode()
    };

    spawn_player_entity(&mut renderer.commands, translation, game_mode);
    runtime.loading_state.transition_requested = true;
    runtime
        .transition
        .request(ScreenTransitionTarget::game(GameState::Gameplay));
}
