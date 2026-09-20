use crate::{
    voxel::mesh_snapshot::ChunkMeshSnapshot,
    world::{
        chunk_mesh_tasks::{ChunkMeshTasks, MAX_MESH_TASKS_IN_FLIGHT},
        chunk_rendering::spawn_built_chunk_meshes,
        chunk_system_params::{ChunkContent, ChunkRenderer},
        work_budget::FrameWorkBudget,
    },
};

use super::{INITIAL_LOADING_BUDGET, super::{WorldLoadingPhase, system_params::WorldSetupProgress}};

pub(super) fn mesh_initial_chunks(
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
