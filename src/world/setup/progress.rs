use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
    voxel::{lighting::initialize_chunks_lighting, world::VoxelWorld},
};

use super::{WorldLoadingPhase, WorldLoadingState};
use crate::world::{
    chunk_loading::ensure_chunk_loaded,
    chunk_rendering::spawn_chunk_mesh,
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    fluid_updates::PendingFluidUpdates,
};

const BOOTSTRAP_LIGHT_BATCH_CHUNKS: usize = 2;
const INITIAL_LOADING_BUDGET: Duration = Duration::from_millis(12);

pub(super) fn setup_world(
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    mut world: ResMut<VoxelWorld>,
    mut loading_state: ResMut<WorldLoadingState>,
    mut transition: ResMut<ScreenTransition>,
    mut fluid_updates: ResMut<PendingFluidUpdates>,
) {
    if transition.is_active() {
        return;
    }

    if !loading_state.screen_rendered {
        loading_state.screen_rendered = true;
        return;
    }

    let generation_context = generation.context(&content);

    match loading_state.phase {
        WorldLoadingPhase::Generating => {
            let frame_started = Instant::now();
            let mut processed = 0;

            loop {
                if processed > 0 && frame_started.elapsed() >= INITIAL_LOADING_BUDGET {
                    break;
                }

                let Some(coord) = loading_state.coords.get(loading_state.generated).copied() else {
                    break;
                };

                ensure_chunk_loaded(&mut world, coord, &generation_context);
                fluid_updates.enqueue_loaded_fluid_frontier(&world, coord);
                loading_state.generated += 1;
                processed += 1;
            }

            if loading_state.generated >= loading_state.coords.len() {
                loading_state.phase = WorldLoadingPhase::Lighting;
            }
        }
        WorldLoadingPhase::Lighting => {
            let frame_started = Instant::now();
            let mut processed = 0;

            loop {
                if processed > 0 && frame_started.elapsed() >= INITIAL_LOADING_BUDGET {
                    break;
                }

                let start = loading_state.lit;
                if start >= loading_state.coords.len() {
                    break;
                }
                let end = (start + BOOTSTRAP_LIGHT_BATCH_CHUNKS).min(loading_state.coords.len());

                drop(initialize_chunks_lighting(
                    &mut world,
                    &loading_state.coords[start..end],
                    &content.blocks,
                    &content.fluids,
                ));
                loading_state.lit = end;
                processed += end - start;
            }

            if loading_state.lit >= loading_state.coords.len() {
                loading_state.phase = WorldLoadingPhase::Meshing;
            }
        }
        WorldLoadingPhase::Meshing => {
            let frame_started = Instant::now();
            let mut processed = 0;

            loop {
                if processed > 0 && frame_started.elapsed() >= INITIAL_LOADING_BUDGET {
                    break;
                }

                let Some(coord) = loading_state.coords.get(loading_state.meshed).copied() else {
                    break;
                };
                let chunk = world
                    .chunk(coord)
                    .unwrap_or_else(|| panic!("generated chunk should exist at {coord:?}"));
                let render_context = content.render_context(
                    &world,
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
                loading_state.meshed += 1;
                processed += 1;
            }

            if loading_state.meshed >= loading_state.coords.len()
                && !loading_state.transition_requested
            {
                loading_state.transition_requested = true;
                transition.request(ScreenTransitionTarget::game(GameState::Gameplay));
            }
        }
    }
}
