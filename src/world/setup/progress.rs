use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::{
        player_id::LOCAL_PLAYER_ID, player_position_is_clear, safe_spawn_position,
        spawn_player_entity,
    },
    ui::transition::ScreenTransitionTarget,
    voxel::lighting::initialize_chunks_lighting,
};

use super::{
    WorldLoadingPhase,
    system_params::{WorldSetupPersistence, WorldSetupRuntime},
};
use crate::world::{
    WorldLoadMode,
    chunk_loading::ensure_chunk_loaded,
    chunk_rendering::spawn_chunk_mesh,
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
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

    let generation_context = generation.context(&content);

    match runtime.loading_state.phase {
        WorldLoadingPhase::Generating => {
            let frame_started = Instant::now();
            let mut processed = 0;

            loop {
                if processed > 0 && frame_started.elapsed() >= INITIAL_LOADING_BUDGET {
                    break;
                }

                let Some(coord) = runtime
                    .loading_state
                    .coords
                    .get(runtime.loading_state.generated)
                    .copied()
                else {
                    break;
                };

                ensure_chunk_loaded(&mut runtime.world, coord, &generation_context);
                runtime
                    .fluid_updates
                    .enqueue_loaded_fluid_frontier(&runtime.world, coord);
                runtime.loading_state.generated += 1;
                processed += 1;
            }

            if runtime.loading_state.generated >= runtime.loading_state.coords.len() {
                runtime.loading_state.phase = WorldLoadingPhase::Lighting;
            }
        }
        WorldLoadingPhase::Lighting => {
            let frame_started = Instant::now();
            let mut processed = 0;

            loop {
                if processed > 0 && frame_started.elapsed() >= INITIAL_LOADING_BUDGET {
                    break;
                }

                let start = runtime.loading_state.lit;
                if start >= runtime.loading_state.coords.len() {
                    break;
                }
                let end = (start + BOOTSTRAP_LIGHT_BATCH_CHUNKS)
                    .min(runtime.loading_state.coords.len());

                drop(initialize_chunks_lighting(
                    &mut runtime.world,
                    &runtime.loading_state.coords[start..end],
                    &content.blocks,
                    &content.fluids,
                    &content.secondary_properties,
                ));
                runtime.loading_state.lit = end;
                processed += end - start;
            }

            if runtime.loading_state.lit >= runtime.loading_state.coords.len() {
                runtime.loading_state.phase = WorldLoadingPhase::Meshing;
            }
        }
        WorldLoadingPhase::Meshing => {
            let frame_started = Instant::now();
            let mut processed = 0;

            loop {
                if processed > 0 && frame_started.elapsed() >= INITIAL_LOADING_BUDGET {
                    break;
                }

                let Some(coord) = runtime
                    .loading_state
                    .coords
                    .get(runtime.loading_state.meshed)
                    .copied()
                else {
                    break;
                };
                let chunk = runtime
                    .world
                    .chunk(coord)
                    .unwrap_or_else(|| panic!("generated chunk should exist at {coord:?}"));
                let render_context = content.render_context(
                    &runtime.world,
                    &renderer.terrain_materials,
                    &renderer.fluid_materials,
                );

                spawn_chunk_mesh(
                    &mut renderer.commands,
                    &mut renderer.meshes,
                    &mut renderer.pool,
                    coord,
                    chunk,
                    &render_context,
                );
                runtime.loading_state.meshed += 1;
                processed += 1;
            }

            if runtime.loading_state.meshed >= runtime.loading_state.coords.len() {
                runtime.loading_state.phase = WorldLoadingPhase::Spawning;
            }
        }
        WorldLoadingPhase::Spawning => {
            if runtime.loading_state.transition_requested {
                return;
            }

            let saved_position = (*persistence.load_mode == WorldLoadMode::Load)
                .then(|| persistence.save.player_position(LOCAL_PLAYER_ID))
                .flatten();
            let translation = saved_position
                .filter(|position| player_position_is_clear(&runtime.world, *position))
                .unwrap_or_else(|| {
                    safe_spawn_position(&runtime.world, runtime.loading_state.spawn_column)
                });
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
    }
}
