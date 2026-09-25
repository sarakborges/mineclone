mod assets;
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
};

use self::{
    assets::wait_for_gameplay_assets,
    fluids::settle_initial_fluids,
    generation::generate_initial_chunks,
    lighting::light_initial_chunks,
    meshing::mesh_initial_chunks,
    spawning::spawn_loaded_world,
};
use super::{
    WorldLoadingPhase,
    system_params::{
        WorldSetupAssets, WorldSetupChunkPipeline, WorldSetupPersistence, WorldSetupProgress,
        WorldSetupSimulation,
    },
};

pub(super) const INITIAL_LOADING_BUDGET: Duration = Duration::from_millis(12);

pub(in crate::world) fn setup_world(
    mut pipeline: WorldSetupChunkPipeline,
    mut progress: WorldSetupProgress,
    mut assets: WorldSetupAssets,
    mut transition: ResMut<ScreenTransition>,
    mut simulation: WorldSetupSimulation,
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

    if progress.loading_state.phase == WorldLoadingPhase::Meshing
        && !pipeline
            .renderer
            .terrain_materials
            .ensure_texture_array_ready(&mut assets.images)
    {
        return;
    }

    match progress.loading_state.phase {
        WorldLoadingPhase::Generating => generate_initial_chunks(
            &pipeline.generation,
            &pipeline.content,
            &mut progress,
            &mut simulation.fluids,
            &mut pipeline.generation_tasks,
            &pipeline.async_work,
            *persistence.load_mode,
        ),
        WorldLoadingPhase::SettlingFluids => {
            settle_initial_fluids(&pipeline.content, &mut progress, &mut simulation.fluids)
        }
        WorldLoadingPhase::Lighting => light_initial_chunks(
            &pipeline.content,
            &mut progress,
            &mut simulation.lighting,
            &mut simulation.changed_lighting_chunks,
        ),
        WorldLoadingPhase::Meshing => {
            mesh_initial_chunks(
                &pipeline.content,
                &mut pipeline.renderer,
                &mut progress,
                &mut pipeline.mesh_tasks,
                &pipeline.async_work,
            )
        }
        WorldLoadingPhase::Assets => wait_for_gameplay_assets(
            &assets.asset_server,
            &assets.gameplay_preloads,
            &mut progress,
        ),
        WorldLoadingPhase::Spawning => spawn_loaded_world(
            &pipeline.content,
            &mut pipeline.renderer,
            &mut progress,
            &mut transition,
            &persistence,
            &player_definition,
        ),
    }
}
