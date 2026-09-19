use std::time::Duration;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::{
        find_safe_spawn_position, player_id::LOCAL_PLAYER_ID, player_position_is_clear,
        safe_spawn_position, spawn_player_entity,
    },
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
    voxel::{lighting::initialize_chunks_lighting, mesh_snapshot::ChunkMeshSnapshot},
};

use super::{
    WorldLoadingPhase,
    system_params::{WorldSetupPersistence, WorldSetupProgress},
};
use crate::world::{
    WorldLoadMode,
    chunk_generation_tasks::{ChunkGenerationTasks, MAX_GENERATION_TASKS_IN_FLIGHT},
    chunk_mesh_tasks::{ChunkMeshTasks, MAX_MESH_TASKS_IN_FLIGHT},
    chunk_rendering::spawn_built_chunk_meshes,
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    fluid_updates::PendingFluidUpdates,
    work_budget::FrameWorkBudget,
};

const BOOTSTRAP_LIGHT_BATCH_CHUNKS: usize = 2;
const INITIAL_LOADING_BUDGET: Duration = Duration::from_millis(12);

#[expect(
    clippy::too_many_arguments,
    reason = "world bootstrap system keeps independently borrowed Bevy resources explicit"
)]
pub(in crate::world) fn setup_world(
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    mut progress: WorldSetupProgress,
    mut transition: ResMut<ScreenTransition>,
    mut fluid_updates: ResMut<PendingFluidUpdates>,
    mut generation_tasks: ResMut<ChunkGenerationTasks>,
    mut mesh_tasks: ResMut<ChunkMeshTasks>,
    persistence: WorldSetupPersistence,
    player_definition: Res<crate::content::player::PlayerDefinition>,
) {
    if transition.is_active() {
        return;
    }

    if !progress.loading_state.screen_rendered {
        progress.loading_state.screen_rendered = true;
        return;
    }

    match progress.loading_state.phase {
        WorldLoadingPhase::Generating => generate_initial_chunks(
            &generation,
            &content,
            &mut progress,
            &mut fluid_updates,
            &mut generation_tasks,
        ),
        WorldLoadingPhase::Lighting => light_initial_chunks(&content, &mut progress),
        WorldLoadingPhase::Meshing => {
            mesh_initial_chunks(&content, &mut renderer, &mut progress, &mut mesh_tasks)
        }
        WorldLoadingPhase::Spawning => spawn_loaded_world(
            &content,
            &mut renderer,
            &mut progress,
            &mut transition,
            &persistence,
            &player_definition,
        ),
    }
}

fn generate_initial_chunks(
    generation: &ChunkGeneration<'_>,
    content: &ChunkContent<'_>,
    progress: &mut WorldSetupProgress<'_>,
    fluid_updates: &mut PendingFluidUpdates,
    generation_tasks: &mut ChunkGenerationTasks,
) {
    generation_tasks.sync_snapshot(generation, content);
    let mut budget = FrameWorkBudget::new(INITIAL_LOADING_BUDGET, 1);

    integrate_generated_chunks(&mut budget, progress, fluid_updates, generation_tasks);
    dispatch_generation_tasks(&mut budget, progress, fluid_updates, generation_tasks);

    if progress.loading_state.generation_cursor >= progress.loading_state.coords.len()
        && progress.loading_state.generated >= progress.loading_state.coords.len()
        && generation_tasks.pending_count() == 0
    {
        progress.loading_state.phase = WorldLoadingPhase::Lighting;
    }
}

fn integrate_generated_chunks(
    budget: &mut FrameWorkBudget,
    progress: &mut WorldSetupProgress<'_>,
    fluid_updates: &mut PendingFluidUpdates,
    generation_tasks: &mut ChunkGenerationTasks,
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
        fluid_updates.enqueue_loaded_fluid_frontier(&progress.world, completed.coord);
        progress.loading_state.generated += 1;
    }
}

fn dispatch_generation_tasks(
    budget: &mut FrameWorkBudget,
    progress: &mut WorldSetupProgress<'_>,
    fluid_updates: &mut PendingFluidUpdates,
    generation_tasks: &mut ChunkGenerationTasks,
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
            fluid_updates.enqueue_loaded_fluid_frontier(&progress.world, coord);
            progress.loading_state.generation_cursor += 1;
            progress.loading_state.generated += 1;
            budget.record(1);
            continue;
        }

        if progress.world.has_generated_chunk(coord) {
            assert!(
                progress.world.restore_chunk(coord),
                "generated bootstrap chunk must be resident or archived: {coord:?}"
            );
            fluid_updates.enqueue_loaded_fluid_frontier(&progress.world, coord);
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

fn light_initial_chunks(content: &ChunkContent<'_>, progress: &mut WorldSetupProgress<'_>) {
    let mut budget = FrameWorkBudget::new(INITIAL_LOADING_BUDGET, 1);

    loop {
        if budget.exhausted() {
            break;
        }

        let start = progress.loading_state.lit;
        if start >= progress.loading_state.coords.len() {
            break;
        }
        let end = (start + BOOTSTRAP_LIGHT_BATCH_CHUNKS).min(progress.loading_state.coords.len());

        drop(initialize_chunks_lighting(
            &mut progress.world,
            &progress.loading_state.coords[start..end],
            content.blocks(),
            content.fluids(),
            content.secondary_properties(),
        ));
        progress.loading_state.lit = end;
        budget.record(end - start);
    }

    if progress.loading_state.lit >= progress.loading_state.coords.len() {
        progress.loading_state.phase = WorldLoadingPhase::Meshing;
    }
}

fn mesh_initial_chunks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    progress: &mut WorldSetupProgress<'_>,
    mesh_tasks: &mut ChunkMeshTasks,
) {
    mesh_tasks.sync_snapshot(content);
    let mut budget = FrameWorkBudget::new(INITIAL_LOADING_BUDGET, 1);

    integrate_built_chunk_meshes(content, renderer, &mut budget, progress, mesh_tasks);
    dispatch_mesh_tasks(content, renderer, &mut budget, progress, mesh_tasks);

    if progress.loading_state.mesh_cursor >= progress.loading_state.coords.len()
        && progress.loading_state.meshed >= progress.loading_state.coords.len()
        && mesh_tasks.pending_count() == 0
    {
        progress.loading_state.phase = WorldLoadingPhase::Spawning;
    }
}

fn integrate_built_chunk_meshes(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    budget: &mut FrameWorkBudget,
    progress: &mut WorldSetupProgress<'_>,
    mesh_tasks: &mut ChunkMeshTasks,
) {
    let current_revision = mesh_tasks.revision();

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = mesh_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        let coord = completed.coord;
        let output = completed.output;
        if completed.revision != current_revision
            || !output.dependencies.is_current(&progress.world)
        {
            let snapshot = ChunkMeshSnapshot::capture(&progress.world, coord)
                .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
            assert!(
                mesh_tasks.schedule(coord, snapshot),
                "stale bootstrap mesh must be rescheduled for {coord:?}"
            );
            continue;
        }

        let render_context = content.render_context(
            &progress.world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );
        spawn_built_chunk_meshes(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            output.meshes,
            &render_context,
        );
        progress.loading_state.meshed += 1;
    }
}

fn dispatch_mesh_tasks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    budget: &mut FrameWorkBudget,
    progress: &mut WorldSetupProgress<'_>,
    mesh_tasks: &mut ChunkMeshTasks,
) {
    loop {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = progress
            .loading_state
            .coords
            .get(progress.loading_state.mesh_cursor)
            .copied()
        else {
            break;
        };
        let chunk_is_empty = progress
            .world
            .chunk(coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"))
            .is_empty();

        if chunk_is_empty {
            let render_context = content.render_context(
                &progress.world,
                &renderer.terrain_materials,
                &renderer.fluid_materials,
            );
            spawn_built_chunk_meshes(
                &mut renderer.commands,
                &mut renderer.meshes,
                &mut renderer.pool,
                coord,
                Vec::new(),
                &render_context,
            );
            progress.loading_state.mesh_cursor += 1;
            progress.loading_state.meshed += 1;
            budget.record(1);
            continue;
        }

        if mesh_tasks.pending_count() >= MAX_MESH_TASKS_IN_FLIGHT {
            break;
        }

        let snapshot = ChunkMeshSnapshot::capture(&progress.world, coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        if !mesh_tasks.schedule(coord, snapshot) {
            break;
        }

        progress.loading_state.mesh_cursor += 1;
        budget.record(1);
    }
}

fn spawn_loaded_world(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    progress: &mut WorldSetupProgress<'_>,
    transition: &mut ScreenTransition,
    persistence: &WorldSetupPersistence<'_>,
    player_definition: &crate::content::player::PlayerDefinition,
) {
    if progress.loading_state.transition_requested {
        return;
    }

    let saved_position = (*persistence.load_mode == WorldLoadMode::Load)
        .then(|| persistence.save.player_position(LOCAL_PLAYER_ID))
        .flatten();
    let translation = saved_position
        .filter(|position| player_position_is_clear(&progress.world, *position))
        .unwrap_or_else(|| spawn_position(content, progress, persistence));
    let game_mode = if *persistence.load_mode == WorldLoadMode::Load {
        persistence.save.player_game_mode(LOCAL_PLAYER_ID)
    } else {
        persistence.new_world_config.game_mode()
    };

    let saved_health = (*persistence.load_mode == WorldLoadMode::Load)
        .then(|| persistence.save.player_health(LOCAL_PLAYER_ID))
        .flatten();
    spawn_player_entity(
        &mut renderer.commands,
        translation,
        game_mode,
        player_definition,
        saved_health,
    );
    progress.loading_state.transition_requested = true;
    transition.request(ScreenTransitionTarget::game(GameState::Gameplay));
}

fn spawn_position(
    content: &ChunkContent<'_>,
    progress: &WorldSetupProgress<'_>,
    persistence: &WorldSetupPersistence<'_>,
) -> Vec3 {
    let preferred_column = progress.loading_state.spawn_column;
    let Some(forced_biome) = (*persistence.load_mode == WorldLoadMode::New)
        .then(|| persistence.new_world_config.spawn_biome())
        .flatten()
    else {
        return safe_spawn_position(&progress.world, preferred_column);
    };

    find_safe_spawn_position(&progress.world, preferred_column, |column| {
        content
            .biome_field
            .sample_surface(column.as_vec2() + Vec2::splat(0.5))
            .primary_id
            == forced_biome
    })
    .unwrap_or_else(|| {
        panic!(
            "could not find a safe generated player spawn in biome {forced_biome} near {preferred_column:?}"
        )
    })
}
