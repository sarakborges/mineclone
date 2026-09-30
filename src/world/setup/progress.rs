mod assets;
mod fluids;
mod generation;
mod lighting;
mod finalization;
mod meshing;
mod spawning;

use std::time::Duration;

use bevy::{ecs::system::Local, prelude::*};

use crate::app::crash_log::log_gameplay_event;

use self::{
    assets::wait_for_gameplay_assets,
    finalization::finalize_initial_world,
    fluids::settle_initial_fluids,
    generation::generate_initial_chunks,
    lighting::light_initial_chunks,
    meshing::mesh_initial_chunks,
    spawning::{InitialPresentationPrewarm, spawn_loaded_world},
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
pub(super) const INITIAL_PRESENTATION_PREWARM_FRAMES: u8 = 12;

#[derive(Default)]
struct LoadingDiagnostics {
    timer: Option<Timer>,
    previous_phase: Option<WorldLoadingPhase>,
}

fn log_loading_diagnostics(
    time: Res<Time<Real>>,
    state: &super::WorldLoadingState,
    diagnostics: &mut LoadingDiagnostics,
) {
    let timer = diagnostics.timer.get_or_insert_with(|| Timer::from_seconds(0.5, TimerMode::Repeating));
    timer.tick(time.delta());

    if diagnostics.previous_phase != Some(state.phase) {
        if let Some(previous) = diagnostics.previous_phase {
            log_gameplay_event(format!("world.loading.phase.complete phase={previous:?}"));
        }
        log_gameplay_event(format!("world.loading.phase.start phase={:?} detail={}", state.phase, loading_phase_detail(state)));
        diagnostics.previous_phase = Some(state.phase);
        return;
    }

    if timer.just_finished() {
        log_gameplay_event(format!("world.loading.phase.progress phase={:?} detail={}", state.phase, loading_phase_detail(state)));
    }
}

fn loading_phase_detail(state: &super::WorldLoadingState) -> String {
    match state.phase {
        WorldLoadingPhase::Generating => format!("generated={}/{} cursor={}", state.generated, state.total(), state.generation_cursor),
        WorldLoadingPhase::SettlingFluids => {
            let (_, generated, mutable, initialization, work, verification, verification_chunks) = state.fluid_settling.diagnostic_counts();
            format!("generated_chunks={generated} mutable_chunks={mutable} initialization={initialization} work={work} verification={verification} verification_chunks={verification_chunks}")
        }
        WorldLoadingPhase::Lighting => format!("seeded={}/{} relaxations={}/{}", state.lighting_seed_cursor, state.total(), state.lighting_relaxation_cursor, state.lighting_relaxations.len()),
        WorldLoadingPhase::Meshing => format!("meshed={}/{} cursor={}", state.meshed, state.total(), state.mesh_cursor),
        WorldLoadingPhase::Assets => format!("loaded={}/{}", state.assets_loaded, state.assets_total),
        WorldLoadingPhase::Finalizing => format!("frames={}/{}", state.finalization_frames, INITIAL_FINALIZATION_FRAMES),
        WorldLoadingPhase::Spawning => format!("presentation_prewarm={}/{}", state.presentation_prewarm_frames, INITIAL_PRESENTATION_PREWARM_FRAMES),
    }
}

pub(in crate::world) fn setup_world(
    time: Res<Time<Real>>,
    mut pipeline: WorldSetupChunkPipeline,
    mut progress: WorldSetupProgress,
    mut assets: WorldSetupAssets,
    mut simulation: WorldSetupSimulation,
    persistence: WorldSetupPersistence,
    mut finalization: WorldSetupFinalization,
    mut initial_presentation_prewarm: Local<InitialPresentationPrewarm>,
    mut loading_diagnostics: Local<LoadingDiagnostics>,
) {
    if finalization.transition.is_active() {
        return;
    }

    if progress.loading_state.phase != WorldLoadingPhase::Spawning {
        initial_presentation_prewarm.reset();
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
        WorldLoadingPhase::SettlingFluids => settle_initial_fluids(
            &pipeline.content,
            &mut progress,
            &mut simulation.fluids,
            &mut loading_diagnostics,
            time,
        ),
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
                &pipeline.lighting_revisions,
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
            &mut initial_presentation_prewarm,
        ),
    }

    log_loading_diagnostics(
        time,
        &progress.loading_state,
        &mut loading_diagnostics,
    );
}
