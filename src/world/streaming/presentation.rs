use std::time::Duration;

use bevy::prelude::*;

use crate::voxel::world::VoxelWorld;

use super::ChunkStreamingState;
use crate::world::{
    chunk_remesh::ChunkRemeshQueue,
    chunk_rendering::{ChunkRenderPool, spawn_built_chunk_meshes},
    chunk_system_params::{ChunkContent, ChunkRenderer},
    chunk_visibility::ChunkPresentationSelection,
    work_budget::{FrameWorkBudget, WorldFrameWorkBudget},
};

const PRESENTATION_PUBLICATION_BUDGET: Duration = Duration::from_millis(1);
const MAX_PRESENTATION_PUBLICATIONS_PER_FRAME: usize = 4;

pub(super) fn publish_pending_chunk_presentations(
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    world: Res<VoxelWorld>,
    selection: Res<ChunkPresentationSelection>,
    mut streaming: ResMut<ChunkStreamingState>,
    mut remesh: ResMut<ChunkRemeshQueue>,
    frame_budget: Res<WorldFrameWorkBudget>,
) {
    let mut budget = FrameWorkBudget::new(PRESENTATION_PUBLICATION_BUDGET, 1)
        .with_global_deadline(frame_budget.deadline())
        .with_maximum_items(MAX_PRESENTATION_PUBLICATIONS_PER_FRAME);

    while !budget.exhausted() {
        let Some(coord) = streaming.pop_presentation_by_priority(&selection) else {
            break;
        };
        budget.record(1);

        if !streaming.keeps_loaded(coord) || renderer.pool.contains(coord) {
            continue;
        }
        let Some(chunk) = world.chunk(coord) else {
            continue;
        };
        let has_geometry = chunk.has_terrain_content();
        let has_fluid = chunk.has_fluid();

        // Reserve presentation membership without synchronously building the
        // chunk mesh. The existing remesh scheduler owns all heavy mesh work.
        let render_context = content.render_context(
            &world,
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

        // A zero dependency offset intentionally selects every meshlet. This
        // turns initial publication into ordinary priority remesh work instead
        // of introducing a second initial-meshing scheduler.
        remesh.enqueue_halo_change(coord, IVec3::ZERO, has_geometry, has_fluid);
        notify_presented_chunk_neighbors(coord, &world, &renderer.pool, &mut remesh);
    }
}

fn notify_presented_chunk_neighbors(
    coord: IVec3,
    world: &VoxelWorld,
    render_pool: &ChunkRenderPool,
    remesh: &mut ChunkRemeshQueue,
) {
    let Some(chunk) = world.chunk(coord) else {
        return;
    };

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

                let new_content_border = chunk.dependency_boundary_has_content(offset);
                let geometry =
                    new_content_border && neighbor_chunk.dependency_boundary_has_content(-offset);
                let neighbor_has_fluid_border =
                    neighbor_chunk.dependency_boundary_has_fluid(-offset);
                let new_cardinal_fluid = offset.x.abs() + offset.y.abs() + offset.z.abs() == 1
                    && chunk.boundary_has_fluid(offset);
                let fluid =
                    (neighbor_has_fluid_border && new_content_border) || new_cardinal_fluid;
                if geometry || fluid {
                    remesh.enqueue_halo_change(neighbor, -offset, geometry, fluid);
                }
            }
        }
    }
}
