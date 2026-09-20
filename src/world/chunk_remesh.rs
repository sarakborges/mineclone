mod queue;

use std::time::Duration;

use bevy::prelude::*;

use crate::voxel::{mesh_snapshot::ChunkMeshSnapshot, world::VoxelWorld};

pub(crate) use self::queue::ChunkRemeshQueue;

use super::{
    chunk_remesh_tasks::{
        ChunkRemeshTaskKind, ChunkRemeshTaskMeshes, ChunkRemeshTasks,
    },
    chunk_rendering::{
        ChunkRenderPool, apply_built_chunk_fluid_meshes, apply_built_chunk_geometry_meshes,
        refresh_chunk_geometry_mesh,
    },
    chunk_system_params::{ChunkContent, ChunkRenderer},
    work_budget::FrameWorkBudget,
};

const REMESH_TASK_DISPATCH_BUDGET: Duration = Duration::from_millis(1);
const REMESH_RESULT_INTEGRATION_BUDGET: Duration = Duration::from_millis(1);
const MAX_REMESH_TASKS_DISPATCHED_PER_FRAME: usize = 4;
const MAX_REMESH_RESULTS_COLLECTED_PER_FRAME: usize = 4;

pub(super) fn process_immediate_geometry_remesh(
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    world: Res<VoxelWorld>,
    mut queue: ResMut<ChunkRemeshQueue>,
) {
    let Some(coord) = queue.pop_renderable_immediate_geometry(&renderer.pool) else {
        return;
    };
    let render_context = content.render_context(
        &world,
        &renderer.terrain_materials,
        &renderer.fluid_materials,
    );

    refresh_chunk_geometry_mesh(
        &mut renderer.commands,
        &mut renderer.meshes,
        &mut renderer.pool,
        coord,
        &render_context,
    );
}

pub(super) fn process_chunk_remesh_queue(
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    world: Res<VoxelWorld>,
    mut queue: ResMut<ChunkRemeshQueue>,
    mut tasks: ResMut<ChunkRemeshTasks>,
    mut deferred: Local<Vec<(IVec3, ChunkRemeshTaskKind)>>,
) {
    tasks.sync_snapshot(&content);

    if tasks.pending_count() > 0 {
        collect_completed_remesh_tasks(
            &content,
            &mut renderer,
            &world,
            &mut queue,
            &mut tasks,
        );
    }

    if !queue.has_background_work() {
        return;
    }

    dispatch_remesh_tasks(
        &world,
        &renderer.pool,
        &mut queue,
        &mut tasks,
        &mut deferred,
    );
}

fn collect_completed_remesh_tasks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    world: &VoxelWorld,
    queue: &mut ChunkRemeshQueue,
    tasks: &mut ChunkRemeshTasks,
) {
    let current_revision = tasks.revision();
    let mut budget = FrameWorkBudget::new(REMESH_RESULT_INTEGRATION_BUDGET, 1)
        .with_maximum_items(MAX_REMESH_RESULTS_COLLECTED_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        let coord = completed.coord;
        let output = completed.output;
        if !renderer.pool.contains(coord) || world.chunk(coord).is_none() {
            continue;
        }
        if completed.revision != current_revision
            || !output.dependencies.content_is_current(world)
        {
            queue.enqueue_task_priority(coord, output.kind);
            continue;
        }

        let lighting_is_current = output.dependencies.lighting_is_current();
        if output.kind != ChunkRemeshTaskKind::Fluid && !lighting_is_current {
            queue.enqueue_task_priority(coord, output.kind);
            continue;
        }

        let render_context = content.render_context(
            world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );
        match output.meshes {
            ChunkRemeshTaskMeshes::Geometry(meshes) => apply_built_chunk_geometry_meshes(
                &mut renderer.commands,
                &mut renderer.meshes,
                &mut renderer.pool,
                coord,
                meshes,
                &render_context,
            ),
            ChunkRemeshTaskMeshes::Fluid(meshes) => {
                apply_built_chunk_fluid_meshes(
                    &mut renderer.commands,
                    &mut renderer.meshes,
                    &mut renderer.pool,
                    coord,
                    meshes,
                    &render_context,
                );
                if !lighting_is_current {
                    queue.enqueue_fluid_priority(coord);
                }
            }
        }
    }
}

fn dispatch_remesh_tasks(
    world: &VoxelWorld,
    render_pool: &ChunkRenderPool,
    queue: &mut ChunkRemeshQueue,
    tasks: &mut ChunkRemeshTasks,
    deferred: &mut Vec<(IVec3, ChunkRemeshTaskKind)>,
) {
    let mut budget = FrameWorkBudget::new(REMESH_TASK_DISPATCH_BUDGET, 1)
        .with_maximum_items(MAX_REMESH_TASKS_DISPATCHED_PER_FRAME);
    deferred.clear();

    loop {
        if budget.exhausted() {
            break;
        }

        let allow_terrain = tasks.can_schedule(ChunkRemeshTaskKind::Geometry);
        let allow_fluid = tasks.can_schedule(ChunkRemeshTaskKind::Fluid);
        if !allow_terrain && !allow_fluid {
            break;
        }

        let Some((coord, kind)) =
            queue.pop_renderable_background(render_pool, allow_terrain, allow_fluid)
        else {
            break;
        };

        if tasks.contains(coord, kind) {
            deferred.push((coord, kind));
            continue;
        }
        let Some(snapshot) = ChunkMeshSnapshot::capture(world, coord) else {
            continue;
        };
        if !tasks.schedule(coord, kind, snapshot) {
            deferred.push((coord, kind));
            continue;
        }
        budget.record(1);
    }

    for (coord, kind) in deferred.drain(..).rev() {
        queue.enqueue_task_priority(coord, kind);
    }
}
