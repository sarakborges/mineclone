mod fluids;
mod generation;
mod lighting;
mod meshing;
mod spawning;

use std::time::Duration;

use bevy::prelude::*;

use crate::{
    content::player::PlayerDefinition,
    ui::transition::ScreenTransition,
    world::{
        chunk_generation_tasks::ChunkGenerationTasks,
        chunk_mesh_tasks::ChunkMeshTasks,
        chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    },
};

use self::{
    fluids::prime_initial_fluids,
    generation::generate_initial_chunks,
    lighting::light_initial_chunks,
    meshing::mesh_initial_chunks,
    spawning::spawn_loaded_world,
};
use super::{
    WorldLoadingPhase,
    system_params::{WorldSetupPersistence, WorldSetupProgress, WorldSetupSimulation},
};

pub(super) const INITIAL_LOADING_BUDGET: Duration = Duration::from_millis(12);

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
    mut simulation: WorldSetupSimulation,
    mut generation_tasks: ResMut<ChunkGenerationTasks>,
    mut mesh_tasks: ResMut<ChunkMeshTasks>,
    persistence: WorldSetupPersistence,
    player_definition: Res<PlayerDefinition>,
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
            &mut simulation.fluids,
            &mut generation_tasks,
            *persistence.load_mode,
        ),
        WorldLoadingPhase::PrimingFluids => {
            prime_initial_fluids(&content, &mut progress, &mut simulation.fluids)
        }
        WorldLoadingPhase::Lighting => light_initial_chunks(
            &content,
            &mut progress,
            &mut simulation.lighting,
            &mut simulation.changed_lighting_chunks,
        ),
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
