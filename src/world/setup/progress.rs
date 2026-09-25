mod assets;
mod fluids;
mod generation;
mod lighting;
mod finalization;
mod meshing;
mod spawning;

use std::time::Duration;

use self::{
    assets::wait_for_gameplay_assets,
    finalization::finalize_initial_world,
    fluids::settle_initial_fluids,
    generation::generate_initial_chunks,
    lighting::light_initial_chunks,
    meshing::mesh_initial_chunks,
    spawning::spawn_loaded_world,
};
use super::{
    WorldLoadingPhase,
    system_params::{
        WorldSetupAssets, WorldSetupChunkPipeline, WorldSetupFinalization, WorldSetupPersistence,
        WorldSetupProgress, WorldSetupSimulation,
    },
};

pub(super) const INITIAL_LOADING_BUDGET: Duration = Duration::from_millis(12);
pub(super) const INITIAL_LOADING_DISPATCH_BUDGET: Duration = Duration::from_millis(2);
pub(super) const INITIAL_FINALIZATION_FRAMES: u8 = 2;

pub(in crate::world) fn setup_world(
    mut pipeline: WorldSetupChunkPipeline,
    mut progress: WorldSetupProgress,
    mut assets: WorldSetupAssets,
    mut simulation: WorldSetupSimulation,
    persistence: WorldSetupPersistence,
    mut finalization: WorldSetupFinalization,
) {
    if finalization.transition.is_active() {
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
        WorldLoadingPhase::Finalizing => finalize_initial_world(
            &pipeline.renderer.pool,
            &mut progress,
            &mut finalization.chunk_entities,
        ),
        WorldLoadingPhase::Spawning => spawn_loaded_world(
            &pipeline.content,
            &mut pipeline.renderer,
            &mut progress,
            &persistence,
            &mut finalization,
        ),
    }
}
