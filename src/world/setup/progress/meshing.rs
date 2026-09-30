use crate::{
    voxel::{mesh_snapshot::ChunkMeshSnapshot, meshlet::ChunkMeshletMask},
    world::{
        chunk_async_work::ChunkAsyncWorkLimiter,
        chunk_mesh_tasks::PresentationScheduler,
        chunk_rendering::spawn_built_chunk_meshes,
        chunk_system_params::{ChunkContent, ChunkRenderer},
        presentation_snapshot::{ChunkPresentationSource, PresentationLightingRevisions},
        render_work_diagnostics::{PresentationPublicationStage, PresentationPublicationTimer},
        work_budget::FrameWorkBudget,
    },
};

use super::{
    INITIAL_LOADING_BUDGET, INITIAL_LOADING_DISPATCH_BUDGET,
    super::{WorldLoadingPhase, system_params::WorldSetupProgress},
};

pub(super) fn mesh_initial_chunks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    progress: &mut WorldSetupProgress<'_>,
    mesh_tasks: &mut PresentationScheduler,
    lighting_revisions: &PresentationLightingRevisions,
    async_work: &ChunkAsyncWorkLimiter,
) {
    mesh_tasks.sync_snapshot(content);
    let mut integration_budget = FrameWorkBudget::new(INITIAL_LOADING_BUDGET, 1);

    integrate_built_chunk_meshes(
        content,
        renderer,
        &mut integration_budget,
        progress,
        mesh_tasks,
        lighting_revisions,
        async_work,
    );

    // Keep the async pool fed even when publishing completed meshes consumes
    // the main-thread integration budget for this loading frame.
    let mut dispatch_budget = FrameWorkBudget::new(INITIAL_LOADING_DISPATCH_BUDGET, 1);
    dispatch_mesh_tasks(
        content,
        renderer,
        &mut dispatch_budget,
        progress,
        mesh_tasks,
        lighting_revisions,
        async_work,
    );

    if progress.loading_state.mesh_cursor >= progress.loading_state.coords.len()
        && progress.loading_state.meshed >= progress.loading_state.coords.len()
        && mesh_tasks.pending_count() == 0
    {
        progress.loading_state.phase = WorldLoadingPhase::Assets;
    }
}

fn integrate_built_chunk_meshes(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    budget: &mut FrameWorkBudget,
    progress: &mut WorldSetupProgress<'_>,
    mesh_tasks: &mut PresentationScheduler,
    lighting_revisions: &PresentationLightingRevisions,
    async_work: &ChunkAsyncWorkLimiter,
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
            || !output.content_source.is_current(&*progress.world)
            || !output.lighting_source.is_current(lighting_revisions)
        {
            let snapshot = ChunkMeshSnapshot::capture(&*progress.world, coord)
                .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
            assert!(
                mesh_tasks.schedule_loading(coord, snapshot, lighting_revisions, async_work),
                "stale bootstrap mesh must be rescheduled for {coord:?}"
            );
            continue;
        }

        let render_context = content.render_context(
            &progress.world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );
        let content_source = output.content_source;
        let lighting_source = output.lighting_source;
        {
            let _publication_timer =
                PresentationPublicationTimer::start(PresentationPublicationStage::InitialPublish);
            spawn_built_chunk_meshes(
                &mut renderer.commands,
                &mut renderer.meshes,
                &mut renderer.pool,
                coord,
                output.meshes,
                &render_context,
            );
            renderer.pool.record_initial_presentation_sources(
                coord,
                content_source,
                lighting_source,
            );
        }
        progress.loading_state.meshed += 1;
    }
}

fn dispatch_mesh_tasks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    budget: &mut FrameWorkBudget,
    progress: &mut WorldSetupProgress<'_>,
    mesh_tasks: &mut PresentationScheduler,
    lighting_revisions: &PresentationLightingRevisions,
    async_work: &ChunkAsyncWorkLimiter,
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
            .unwrap_or_else(|| panic!("generated bootstrap chunk data should exist at {coord:?}"))
            .is_empty();

        if chunk_is_empty {
            let content_source = ChunkPresentationSource::capture_center(coord, &*progress.world)
                .unwrap_or_else(|| panic!("empty bootstrap chunk source should exist at {coord:?}"));
            let lighting_source = lighting_revisions.capture(coord, ChunkMeshletMask::ALL);
            let render_context = content.render_context(
                &progress.world,
                &renderer.terrain_materials,
                &renderer.fluid_materials,
            );
            {
                let _publication_timer =
                    PresentationPublicationTimer::start(PresentationPublicationStage::InitialPublish);
                spawn_built_chunk_meshes(
                    &mut renderer.commands,
                    &mut renderer.meshes,
                    &mut renderer.pool,
                    coord,
                    Vec::new(),
                    &render_context,
                );
                renderer.pool.record_initial_presentation_sources(
                    coord,
                    content_source,
                    lighting_source,
                );
            }
            progress.loading_state.mesh_cursor += 1;
            progress.loading_state.meshed += 1;
            budget.record(1);
            continue;
        }

        if mesh_tasks.pending_count() >= async_work.loading_queue_limit() {
            break;
        }

        let snapshot = ChunkMeshSnapshot::capture(&*progress.world, coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        if !mesh_tasks.schedule_loading(coord, snapshot, lighting_revisions, async_work) {
            break;
        }

        progress.loading_state.mesh_cursor += 1;
        budget.record(1);
    }
}
