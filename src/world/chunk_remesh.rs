mod queue;

use std::time::{Duration, Instant};

use bevy::{platform::collections::HashMap, prelude::*};

use crate::voxel::{
    mesh_snapshot::ChunkMeshSnapshot,
    meshlet::ChunkMeshletMask,
    world::VoxelWorld,
};

pub(crate) use self::queue::ChunkRemeshQueue;

use super::{
    chunk_async_work::ChunkAsyncWorkLimiter,
    chunk_remesh_tasks::{
        ChunkRemeshPublication, ChunkRemeshTaskKind, ChunkRemeshTaskMeshes, ChunkRemeshTasks,
    },
    chunk_rendering::{
        ChunkRenderPool, apply_built_chunk_fluid_meshlets,
        apply_built_chunk_geometry_meshlets,
    },
    chunk_system_params::{ChunkContent, ChunkRenderer},
    streaming::ChunkStreamingState,
    work_budget::{FrameWorkBudget, WorldFrameWorkBudget},
};

const REMESH_TASK_DISPATCH_BUDGET: Duration = Duration::from_millis(1);
const REMESH_RESULT_INTEGRATION_BUDGET: Duration = Duration::from_millis(1);
const MAX_REMESH_DISPATCH_ATTEMPTS_PER_FRAME: usize = 4;
const MAX_REMESH_RESULTS_COLLECTED_PER_FRAME: usize = 4;

#[allow(clippy::too_many_arguments)]
pub(super) fn process_chunk_remesh_queue(
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    world: Res<VoxelWorld>,
    mut queue: ResMut<ChunkRemeshQueue>,
    mut tasks: ResMut<ChunkRemeshTasks>,
    frame_budget: Res<WorldFrameWorkBudget>,
    async_work: Res<ChunkAsyncWorkLimiter>,
    streaming: Res<ChunkStreamingState>,
    mut deferred: Local<Vec<(IVec3, ChunkRemeshTaskKind, ChunkMeshletMask)>>,
) {
    tasks.sync_snapshot(&content);

    if tasks.pending_count() > 0 {
        collect_completed_remesh_tasks(
            &content,
            &mut renderer,
            &world,
            &mut queue,
            &mut tasks,
            frame_budget.deadline(),
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
        &async_work,
        streaming.center(),
        frame_budget.deadline(),
    );
}

fn collect_completed_remesh_tasks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    world: &VoxelWorld,
    queue: &mut ChunkRemeshQueue,
    tasks: &mut ChunkRemeshTasks,
    deadline: Instant,
) {
    let current_revision = tasks.revision();
    let mut budget = FrameWorkBudget::new(REMESH_RESULT_INTEGRATION_BUDGET, 1)
        .with_global_deadline(deadline)
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
        let kind = output.kind;
        let meshlets = output.meshlets;
        if !renderer.pool.contains(coord) || world.chunk(coord).is_none() {
            continue;
        }
        if completed.revision != current_revision {
            queue.enqueue_task_meshlets_priority(coord, kind, meshlets);
            continue;
        }

        match output.dependencies.publication(kind, world, tasks) {
            ChunkRemeshPublication::Ready => {}
            ChunkRemeshPublication::Retry(retry_kind) => {
                queue.enqueue_task_meshlets_priority(coord, retry_kind, meshlets);
                continue;
            }
            ChunkRemeshPublication::FluidWithLightingCatchup => {
                queue.enqueue_task_meshlets_priority(coord, kind, meshlets);
            }
        }

        let render_context = content.render_context(
            world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );
        let applied = match output.meshes {
            ChunkRemeshTaskMeshes::Geometry(meshes) => {
                apply_built_chunk_geometry_meshlets(
                    &mut renderer.commands,
                    &mut renderer.meshes,
                    &mut renderer.pool,
                    coord,
                    meshes,
                    meshlets,
                    &render_context,
                )
            }
            ChunkRemeshTaskMeshes::Fluid(meshes) => {
                apply_built_chunk_fluid_meshlets(
                    &mut renderer.commands,
                    &mut renderer.meshes,
                    &mut renderer.pool,
                    coord,
                    meshes,
                    meshlets,
                    &render_context,
                )
            }
        };
        if !applied {
            queue.enqueue_task_priority(coord, kind);
        }
    }
}

fn dispatch_remesh_tasks(
    world: &VoxelWorld,
    render_pool: &ChunkRenderPool,
    queue: &mut ChunkRemeshQueue,
    tasks: &mut ChunkRemeshTasks,
    deferred: &mut Vec<(IVec3, ChunkRemeshTaskKind, ChunkMeshletMask)>,
    async_work: &ChunkAsyncWorkLimiter,
    center: Option<IVec3>,
    deadline: Instant,
) {
    let mut budget = FrameWorkBudget::new(REMESH_TASK_DISPATCH_BUDGET, 1)
        .with_global_deadline(deadline)
        .with_maximum_items(MAX_REMESH_DISPATCH_ATTEMPTS_PER_FRAME);
    deferred.clear();
    let mut snapshots = HashMap::<(IVec3, ChunkMeshletMask), ChunkMeshSnapshot>::new();

    loop {
        if budget.exhausted() {
            break;
        }

        let allow_terrain = tasks.can_schedule(ChunkRemeshTaskKind::Geometry);
        let allow_fluid = tasks.can_schedule(ChunkRemeshTaskKind::Fluid);
        if !allow_terrain && !allow_fluid {
            break;
        }

        let Some((coord, kind, meshlets)) =
            queue.pop_renderable_background(render_pool, center, allow_terrain, allow_fluid)
        else {
            break;
        };
        // Deferred and stale candidates also consume main-thread work. Charge
        // the attempt before any early return can bypass the frame budget.
        budget.record(1);
        let (kind, meshlets) =
            queue.coalesce_terrain_work(coord, kind, meshlets);

        if tasks.contains(coord, kind) {
            deferred.push((coord, kind, meshlets));
            continue;
        }
        let snapshot_key = (coord, meshlets);
        let snapshot = if let Some(existing) = snapshots.get(&snapshot_key) {
            existing.clone()
        } else {
            let Some(captured) =
                ChunkMeshSnapshot::capture_with_neighbor_filter_and_meshlets(
                    world,
                    coord,
                    |neighbor| render_pool.contains(neighbor),
                    meshlets,
                )
            else {
                continue;
            };
            snapshots.insert(snapshot_key, captured.clone());
            captured
        };
        if !tasks.schedule(coord, kind, meshlets, snapshot, async_work) {
            deferred.push((coord, kind, meshlets));
            // The per-kind and per-coordinate checks passed above, so shared
            // executor capacity is exhausted. Preserve the remaining queue.
            break;
        }
    }

    for (coord, kind, meshlets) in deferred.drain(..).rev() {
        queue.enqueue_task_meshlets_priority(coord, kind, meshlets);
    }
}
