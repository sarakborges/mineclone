mod queue;

use std::time::{Duration, Instant};

use bevy::{platform::collections::HashMap, prelude::*};

use crate::voxel::{
    coordinates::visit_chunk_coords_whose_voxel_halo_contains,
    mesh_snapshot::ChunkMeshSnapshot,
    meshlet::ChunkMeshletMask,
    world::VoxelWorld,
};

pub(crate) use self::queue::ChunkRemeshQueue;

pub(crate) fn prune_absent_remesh_halo(
    world_position: IVec3,
    world: &VoxelWorld,
    queue: &mut ChunkRemeshQueue,
) {
    visit_chunk_coords_whose_voxel_halo_contains(world_position, |coord| {
        if world.chunk(coord).is_none() {
            queue.remove(coord);
        }
    });
}

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
    presentation_snapshot::PresentationLightingRevisions,
    streaming::ChunkStreamingState,
    work_budget::{FrameWorkBudget, WorldFrameWorkBudget},
};

const REMESH_TASK_DISPATCH_BUDGET: Duration = Duration::from_millis(1);
const REMESH_RESULT_INTEGRATION_BUDGET: Duration = Duration::from_millis(1);
const MAX_REMESH_DISPATCH_ATTEMPTS_PER_FRAME: usize = 4;
const MAX_REMESH_RESULTS_COLLECTED_PER_FRAME: usize = 4;

struct RemeshDispatchContext<'a> {
    world: &'a VoxelWorld,
    render_pool: &'a ChunkRenderPool,
    streaming: &'a ChunkStreamingState,
    lighting_revisions: &'a PresentationLightingRevisions,
    async_work: &'a ChunkAsyncWorkLimiter,
    center: Option<IVec3>,
    deadline: Instant,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn process_chunk_remesh_queue(
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    world: Res<VoxelWorld>,
    mut queue: ResMut<ChunkRemeshQueue>,
    mut tasks: ResMut<ChunkRemeshTasks>,
    mut lighting_revisions: ResMut<PresentationLightingRevisions>,
    frame_budget: Res<WorldFrameWorkBudget>,
    async_work: Res<ChunkAsyncWorkLimiter>,
    streaming: Res<ChunkStreamingState>,
    mut deferred: Local<Vec<(IVec3, ChunkRemeshTaskKind, ChunkMeshletMask)>>,
    mut last_reconciled_selection: Local<Option<u64>>,
) {
    tasks.sync_snapshot(&content);
    for coord in tasks.drain_lighting_revision_removals() {
        lighting_revisions.remove(coord);
    }

    let selection_revision = streaming.selection_revision();
    // Normal chunk and render retirement remove their remesh entries at the
    // point residency actually changes. A selection revision alone does not
    // make queued work nonresident, so scanning every remesh queue here makes
    // travel cost scale with backlog outside the frame budget. Keep the full
    // reconciliation only for a revision reset, which indicates a world/
    // streaming lifecycle restart while this system-local state survived.
    if last_reconciled_selection
        .is_some_and(|previous| selection_revision < previous)
        && queue.has_background_work()
    {
        queue.retain_resident(&world);
    }
    if *last_reconciled_selection != Some(selection_revision) {
        for request in tasks.cancel_where(|coord| !streaming.retains_render_mesh(coord)) {
            if renderer.pool.contains(request.coord) && world.chunk(request.coord).is_some() {
                queue.enqueue_task_meshlets_priority(
                    request.coord,
                    request.kind,
                    request.meshlets,
                );
            }
        }
        *last_reconciled_selection = Some(selection_revision);
    }

    if tasks.pending_count() > 0 {
        collect_completed_remesh_tasks(
            &content,
            &mut renderer,
            &world,
            &streaming,
            &mut queue,
            &mut tasks,
            &lighting_revisions,
            frame_budget.deadline(),
        );
    }

    if !queue.has_background_work() {
        return;
    }

    // Initial publication is foreground work. While any chunk inside the
    // current show radius is still pending/generated/ready, do not start new
    // background remesh tasks. Completed remeshes above are still integrated,
    // but fresh async capacity is reserved for closing visible terrain holes.
    if streaming.has_renderable_streaming_backlog() {
        return;
    }

    dispatch_remesh_tasks(
        RemeshDispatchContext {
            world: &world,
            render_pool: &renderer.pool,
            streaming: &streaming,
            lighting_revisions: &lighting_revisions,
            async_work: &async_work,
            center: streaming.center(),
            deadline: frame_budget.deadline(),
        },
        &mut queue,
        &mut tasks,
        &mut deferred,
    );
}

fn collect_completed_remesh_tasks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    world: &VoxelWorld,
    streaming: &ChunkStreamingState,
    queue: &mut ChunkRemeshQueue,
    tasks: &mut ChunkRemeshTasks,
    lighting_revisions: &PresentationLightingRevisions,
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
        if !streaming.retains_render_mesh(coord) {
            // Selection can reverse before the budgeted render-retirement pass
            // reaches this allocation. Preserve the dirty meshlets until the
            // allocation is actually retired, or until the chunk re-enters the
            // render residency and becomes eligible for publication again.
            queue.enqueue_task_meshlets_priority(coord, kind, meshlets);
            continue;
        }
        if completed.revision != current_revision {
            queue.enqueue_task_meshlets_priority(coord, kind, meshlets);
            continue;
        }

        match output
            .dependencies
            .publication(kind, world, lighting_revisions)
        {
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
    context: RemeshDispatchContext<'_>,
    queue: &mut ChunkRemeshQueue,
    tasks: &mut ChunkRemeshTasks,
    deferred: &mut Vec<(IVec3, ChunkRemeshTaskKind, ChunkMeshletMask)>,
) {
    let mut budget = FrameWorkBudget::new(REMESH_TASK_DISPATCH_BUDGET, 1)
        .with_global_deadline(context.deadline)
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
            queue.pop_renderable_background(
                context.render_pool,
                context.center,
                allow_terrain,
                allow_fluid,
            )
        else {
            break;
        };
        // Deferred and stale candidates also consume main-thread work. Charge
        // the attempt before any early return can bypass the frame budget.
        budget.record(1);
        if !context.streaming.retains_render_mesh(coord) {
            // Keep invalidation attached to a still-live render allocation.
            // Explicit render retirement removes it; a rapid reversal/warp
            // can instead make it eligible again without losing dirty work.
            deferred.push((coord, kind, meshlets));
            continue;
        }
        let (kind, meshlets) = queue.coalesce_terrain_work(coord, kind, meshlets);

        if tasks.contains(coord, kind) {
            deferred.push((coord, kind, meshlets));
            continue;
        }
        let snapshot_key = (coord, meshlets);
        let snapshot = if let Some(existing) = snapshots.get(&snapshot_key) {
            existing.clone()
        } else {
            let Some(captured) = ChunkMeshSnapshot::capture_with_neighbor_filter_and_meshlets(
                context.world,
                coord,
                |neighbor| context.render_pool.contains(neighbor),
                meshlets,
            ) else {
                continue;
            };
            snapshots.insert(snapshot_key, captured.clone());
            captured
        };
        if !tasks.schedule(
            coord,
            kind,
            meshlets,
            snapshot,
            context.lighting_revisions,
            context.async_work,
        ) {
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
