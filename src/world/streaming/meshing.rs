use std::time::Duration;

use bevy::prelude::*;

use crate::{
    voxel::{mesh_snapshot::ChunkMeshSnapshot, world::VoxelWorld},
    world::{
        chunk_mesh_tasks::MAX_MESH_TASKS_IN_FLIGHT,
        chunk_remesh::ChunkRemeshQueue,
        chunk_rendering::{ChunkRenderPool, spawn_built_chunk_meshes},
        chunk_system_params::{ChunkContent, ChunkRenderer},
        work_budget::FrameWorkBudget,
    },
};

use super::{
    ChunkStreamingQueues, ChunkStreamingWork, is_critical_streaming_coord,
    seed_loaded_chunk_lighting,
};

const MIN_CHUNKS_BEFORE_BUDGET_CHECK: usize = 1;
const MAX_CHUNKS_PER_FRAME: usize = 4;
const MAX_MESH_RESULTS_COLLECTED_PER_FRAME: usize = 4;
const MESH_RESULT_INTEGRATION_BUDGET: Duration = Duration::from_millis(2);
const STREAMING_BUDGET: Duration = Duration::from_millis(4);

pub(super) fn dispatch_initial_mesh_tasks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
    let mut budget = FrameWorkBudget::new(STREAMING_BUDGET, MIN_CHUNKS_BEFORE_BUDGET_CHECK)
        .with_maximum_items(MAX_CHUNKS_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = work.state.pop_ready() else {
            break;
        };
        if !work.state.keeps_loaded(coord) || renderer.pool.contains(coord) {
            continue;
        }
        if work.mesh_tasks.contains(coord) {
            continue;
        }
        let Some(chunk_is_empty) = work.world.chunk(coord).map(|chunk| chunk.is_empty()) else {
            work.state.requeue(coord);
            continue;
        };

        seed_loaded_chunk_lighting(coord, content, work, queues, current_tick);

        if !chunk_is_empty && work.mesh_tasks.pending_count() >= MAX_MESH_TASKS_IN_FLIGHT {
            let Some(center) = work.state.center else {
                work.state.defer_ready(coord);
                break;
            };
            if !is_critical_streaming_coord(coord, center) {
                work.state.defer_ready(coord);
                break;
            }
            let Some(preempted) = work.mesh_tasks.cancel_farthest_where(center, |task_coord| {
                !is_critical_streaming_coord(task_coord, center)
            }) else {
                work.state.defer_ready(coord);
                break;
            };
            work.state.mark_ready(preempted);
        }

        if chunk_is_empty {
            integrate_empty_chunk(content, renderer, &work.world, &mut queues.remesh, coord);
            budget.record(1);
            continue;
        }

        let snapshot = ChunkMeshSnapshot::capture_with_neighbor_filter(
            &work.world,
            coord,
            |neighbor| renderer.pool.contains(neighbor),
        )
        .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        if !work.mesh_tasks.schedule(coord, snapshot) {
            work.state.defer_ready(coord);
            break;
        }

        budget.record(1);
    }
}

fn integrate_empty_chunk(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    world: &VoxelWorld,
    remesh_queue: &mut ChunkRemeshQueue,
    coord: IVec3,
) {
    let render_context = content.render_context(
        world,
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
    notify_loaded_chunk_neighbors(coord, world, &renderer.pool, remesh_queue);
}

pub(super) fn collect_built_chunk_meshes(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    work: &mut ChunkStreamingWork<'_>,
    remesh_queue: &mut ChunkRemeshQueue,
) {
    let current_revision = work.mesh_tasks.revision();
    let mut budget = FrameWorkBudget::new(MESH_RESULT_INTEGRATION_BUDGET, 1)
        .with_maximum_items(MAX_MESH_RESULTS_COLLECTED_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = work.mesh_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        if completed.revision != current_revision {
            work.state.mark_ready(completed.coord);
            continue;
        }
        if renderer.pool.contains(completed.coord) {
            continue;
        }
        if !work.state.retains_render_mesh(completed.coord) {
            work.state.mark_ready(completed.coord);
            continue;
        }
        if !completed.output.dependencies.is_current(&work.world) {
            work.state.mark_ready(completed.coord);
            continue;
        }
        let Some(chunk) = work.world.chunk(completed.coord) else {
            work.state.requeue(completed.coord);
            continue;
        };
        let chunk_has_fluid = chunk.has_fluid();
        let catchup = completed
            .output
            .dependencies
            .needs_initial_catchup_with(&work.world, |neighbor| {
                renderer.pool.contains(neighbor)
            })
            || work.state.initial_mesh_seed_catchup.contains(&completed.coord);
        let render_context = content.render_context(
            &work.world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );

        spawn_built_chunk_meshes(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            completed.coord,
            completed.output.meshes,
            &render_context,
        );
        work.state.initial_mesh_seed_catchup.remove(&completed.coord);
        if catchup {
            remesh_queue.enqueue_priority(completed.coord);
            if chunk_has_fluid {
                remesh_queue.enqueue_fluid_priority(completed.coord);
            }
        }
        notify_loaded_chunk_neighbors(
            completed.coord,
            &work.world,
            &renderer.pool,
            remesh_queue,
        );
    }
}

fn boundary_faces_toward(offset: IVec3, mut has_face: impl FnMut(IVec3) -> bool) -> bool {
    (offset.x == 0 || has_face(IVec3::new(-offset.x, 0, 0)))
        && (offset.y == 0 || has_face(IVec3::new(0, -offset.y, 0)))
        && (offset.z == 0 || has_face(IVec3::new(0, 0, -offset.z)))
}

fn notify_loaded_chunk_neighbors(
    coord: IVec3,
    world: &VoxelWorld,
    render_pool: &ChunkRenderPool,
    remesh_queue: &mut ChunkRemeshQueue,
) {
    let chunk = world
        .chunk(coord)
        .unwrap_or_else(|| panic!("rendered chunk data should exist at {coord:?}"));

    for y in -1..=1 {
        for z in -1..=1 {
            for x in -1..=1 {
                let offset = IVec3::new(x, y, z);
                if offset == IVec3::ZERO {
                    continue;
                }
                let neighbor = coord + offset;
                if !render_pool.contains(neighbor) {
                    continue;
                }
                let Some(neighbor_chunk) = world.chunk(neighbor) else {
                    continue;
                };

                if boundary_faces_toward(offset, |face| neighbor_chunk.boundary_has_content(face)) {
                    remesh_queue.enqueue_priority(neighbor);
                }

                let has_fluid_border = boundary_faces_toward(offset, |face| {
                    neighbor_chunk.boundary_has_fluid(face)
                });
                let new_cardinal_fluid = offset.x.abs() + offset.y.abs() + offset.z.abs() == 1
                    && chunk.boundary_has_fluid(offset);
                if has_fluid_border || new_cardinal_fluid {
                    remesh_queue.enqueue_fluid_priority(neighbor);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{
        cell::VoxelCell, chunk::VoxelChunk, texture_rotation::TextureRotation,
    };

    #[test]
    fn diagonal_boundary_reconciliation_is_restricted_to_relevant_faces() {
        let mut chunk = VoxelChunk::empty();
        chunk.set_block(
            0,
            0,
            7,
            Some(VoxelCell::new("asteria:test", TextureRotation::default())),
        );
        assert!(boundary_faces_toward(IVec3::new(1, 1, 0), |face| {
            chunk.boundary_has_content(face)
        }));
        assert!(!boundary_faces_toward(IVec3::new(-1, 1, 0), |face| {
            chunk.boundary_has_content(face)
        }));
    }
}
