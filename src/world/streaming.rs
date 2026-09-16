mod selection;
mod surface_cache;

use std::time::Duration;

use bevy::{
    ecs::system::SystemParam,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::{
    content::{
        biome::BiomeRegistry, dimension::DimensionDefinition, structure::StructureRegistry,
    },
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::chunk_coord_from_position,
        deduplicated_queue::DeduplicatedQueue,
        lighting::{PendingLightingUpdates, seed_chunk_direct_lighting},
        mesh_snapshot::ChunkMeshSnapshot,
        neighbors::CARDINAL_NEIGHBORS,
        world::VoxelWorld,
    },
};

use self::selection::rebuild_queue;
use super::{
    biome_field::BiomeField,
    chunk_generation_tasks::{ChunkGenerationTasks, MAX_GENERATION_TASKS_IN_FLIGHT},
    chunk_mesh_tasks::{ChunkMeshTasks, MAX_MESH_TASKS_IN_FLIGHT},
    chunk_remesh::ChunkRemeshQueue,
    chunk_rendering::{ChunkRenderPool, spawn_built_chunk_meshes},
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    fluid_updates::PendingFluidUpdates,
    render_distance::RenderDistanceSettings,
    work_budget::FrameWorkBudget,
    world_feature_fields::WorldFeatureFields,
};

const MIN_CHUNKS_BEFORE_BUDGET_CHECK: usize = 1;
const MAX_CHUNKS_PER_FRAME: usize = 4;
const MAX_GENERATION_DISPATCH_WORK_PER_FRAME: usize = 16;
const MAX_GENERATION_TASKS_WITH_MESH_BACKLOG: usize = 4;
const MAX_GENERATION_RESULTS_COLLECTED_PER_FRAME: usize = 8;
const MAX_MESH_RESULTS_COLLECTED_PER_FRAME: usize = 4;
const CRITICAL_PLAYER_RADIUS_CHUNKS: i32 = 1;
const GENERATION_DISPATCH_BUDGET: Duration = Duration::from_millis(1);
const GENERATION_RESULT_INTEGRATION_BUDGET: Duration = Duration::from_millis(1);
const MESH_RESULT_INTEGRATION_BUDGET: Duration = Duration::from_millis(2);
const STREAMING_BUDGET: Duration = Duration::from_millis(4);

#[derive(Resource, Default)]
pub(super) struct ChunkStreamingState {
    center: Option<IVec3>,
    movement_direction: IVec2,
    horizontal_radius: i32,
    vertical_radius: i32,
    desired: HashSet<IVec3>,
    retained: HashSet<IVec3>,
    retired: DeduplicatedQueue<IVec3>,
    pending: DeduplicatedQueue<IVec3>,
    ready: DeduplicatedQueue<IVec3>,
    surface_ranges: HashMap<IVec2, (i32, i32)>,
    initial_lighting_seeded: HashSet<IVec3>,
}

impl ChunkStreamingState {
    pub(super) fn keeps_loaded(&self, coord: IVec3) -> bool {
        self.desired.contains(&coord) || self.retained.contains(&coord)
    }

    pub(super) fn enqueue_retired(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.retired.enqueue(coord);
        }
    }

    pub(super) fn pop_retired_outside_horizontal_radius(
        &mut self,
        center: IVec3,
        horizontal_radius: i32,
    ) -> Option<IVec3> {
        let center = center.xz();
        let radius_squared = horizontal_radius.max(0).pow(2);
        let desired = &self.desired;
        let retained = &self.retained;

        self.retired.pop_where(|coord| {
            if desired.contains(&coord) || retained.contains(&coord) {
                return false;
            }

            let delta = coord.xz() - center;
            delta.length_squared() > radius_squared
        })
    }

    fn requeue(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.pending.contains(coord) && !self.ready.contains(coord) {
            self.pending.enqueue_front(coord);
        }
    }

    fn defer_pending(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.pending.contains(coord) && !self.ready.contains(coord) {
            self.pending.enqueue(coord);
        }
    }

    fn pop_critical_pending(&mut self) -> Option<IVec3> {
        let center = self.center?;
        self.pending
            .pop_where(|coord| is_critical_streaming_coord(coord, center))
    }

    fn mark_ready(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.ready.contains(coord) {
            self.ready.enqueue(coord);
        }
    }

    fn pop_ready(&mut self) -> Option<IVec3> {
        let center = self.center?;
        let movement_direction = self.movement_direction;
        self.ready
            .pop_where(|coord| is_critical_streaming_coord(coord, center))
            .or_else(|| {
                if movement_direction == IVec2::ZERO {
                    None
                } else {
                    self.ready.pop_where(|coord| {
                        (coord.xz() - center.xz()).dot(movement_direction) > 0
                    })
                }
            })
            .or_else(|| self.ready.pop())
    }

    fn defer_ready(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.ready.contains(coord) {
            self.ready.enqueue_front(coord);
        }
    }

    fn mark_initial_lighting_seeded(&mut self, coord: IVec3) -> bool {
        self.initial_lighting_seeded.insert(coord)
    }

    pub(super) fn forget_initial_lighting_seeded(&mut self, coord: IVec3) {
        self.initial_lighting_seeded.remove(&coord);
    }
}

fn is_critical_streaming_coord(coord: IVec3, center: IVec3) -> bool {
    let delta = coord - center;
    delta.x.abs() <= CRITICAL_PLAYER_RADIUS_CHUNKS
        && delta.y.abs() <= CRITICAL_PLAYER_RADIUS_CHUNKS
        && delta.z.abs() <= CRITICAL_PLAYER_RADIUS_CHUNKS
}

struct QueueRebuildContext<'a> {
    render_pool: &'a ChunkRenderPool,
    dimension: &'a DimensionDefinition,
    biomes: &'a BiomeRegistry,
    structures: &'a StructureRegistry,
    biome_field: &'a BiomeField,
    feature_fields: &'a WorldFeatureFields,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingWork<'w> {
    world: ResMut<'w, VoxelWorld>,
    state: ResMut<'w, ChunkStreamingState>,
    generation_tasks: ResMut<'w, ChunkGenerationTasks>,
    mesh_tasks: ResMut<'w, ChunkMeshTasks>,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingSelection<'w, 's> {
    render_distance: Res<'w, RenderDistanceSettings>,
    scratch: Local<'s, selection::QueueRebuildScratch>,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingQueues<'w> {
    remesh: ResMut<'w, ChunkRemeshQueue>,
    fluid: ResMut<'w, PendingFluidUpdates>,
    lighting: ResMut<'w, PendingLightingUpdates>,
}

pub(super) fn stream_chunks(
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    player: Single<&Transform, With<GameplayCamera>>,
    mut selection: ChunkStreamingSelection,
    mut work: ChunkStreamingWork,
    mut queues: ChunkStreamingQueues,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = chunk_coord_from_position(feet_position);
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = selection.render_distance.chunks();
    let vertical_radius = selection.render_distance.vertical_chunks();

    if work.state.center != Some(center)
        || work.state.horizontal_radius != horizontal_radius
        || work.state.vertical_radius != vertical_radius
    {
        let rebuild_context = QueueRebuildContext {
            render_pool: &renderer.pool,
            dimension: generation.dimension(),
            biomes: &content.biomes,
            structures: &generation.structures,
            biome_field: &content.biome_field,
            feature_fields: &generation.feature_fields,
        };
        rebuild_queue(
            &mut work.state,
            center,
            horizontal_radius,
            vertical_radius,
            &mut selection.scratch,
            &rebuild_context,
        );
    }

    work.generation_tasks.sync_snapshot(&generation, &content);
    work.generation_tasks.sync_streaming_region(center);
    work.mesh_tasks.sync_snapshot(&content);

    if work.generation_tasks.pending_count() > 0 {
        collect_generated_chunks(&mut work);
    }
    if work.mesh_tasks.pending_count() > 0 {
        collect_built_chunk_meshes(&content, &mut renderer, &mut work, &mut queues.remesh);
    }
    if work.state.ready.len() > 0 {
        dispatch_initial_mesh_tasks(&content, &mut renderer, &mut work, &mut queues);
    }
    if work.state.pending.len() > 0 {
        dispatch_generation_tasks(&renderer.pool, &mut work);
    }
}

fn collect_generated_chunks(work: &mut ChunkStreamingWork<'_>) {
    let current_revision = work.generation_tasks.revision();
    let mut budget = FrameWorkBudget::new(GENERATION_RESULT_INTEGRATION_BUDGET, 1)
        .with_maximum_items(MAX_GENERATION_RESULTS_COLLECTED_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = work.generation_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        if completed.revision != current_revision {
            work.state.requeue(completed.coord);
            continue;
        }
        if !work.state.keeps_loaded(completed.coord) {
            continue;
        }
        if work.world.has_generated_chunk(completed.coord) {
            work.state.mark_ready(completed.coord);
            continue;
        }

        work.world.insert_chunk(completed.coord, completed.output);
        work.state.mark_ready(completed.coord);
    }
}

fn dispatch_generation_tasks(render_pool: &ChunkRenderPool, work: &mut ChunkStreamingWork<'_>) {
    let max_in_flight = if work.mesh_tasks.pending_count() > 0 {
        MAX_GENERATION_TASKS_WITH_MESH_BACKLOG
    } else {
        MAX_GENERATION_TASKS_IN_FLIGHT
    };
    let mut budget = FrameWorkBudget::new(GENERATION_DISPATCH_BUDGET, 1)
        .with_maximum_items(MAX_GENERATION_DISPATCH_WORK_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let at_capacity = work.generation_tasks.pending_count() >= max_in_flight;
        let coord = if at_capacity {
            work.state.pop_critical_pending()
        } else {
            work.state.pending.pop()
        };
        let Some(coord) = coord else {
            break;
        };

        if render_pool.contains(coord)
            || work.state.ready.contains(coord)
            || work.mesh_tasks.contains(coord)
        {
            continue;
        }
        if work.generation_tasks.contains(coord) {
            continue;
        }

        if work.world.has_generated_chunk(coord) {
            assert!(
                work.world.restore_chunk(coord),
                "generated chunk must be resident or archived: {coord:?}"
            );
            work.state.mark_ready(coord);
            budget.record(1);
            continue;
        }

        if work.generation_tasks.pending_count() >= max_in_flight {
            let Some(center) = work.state.center else {
                work.state.requeue(coord);
                break;
            };
            let Some(preempted) = work
                .generation_tasks
                .cancel_farthest_where(center, |task_coord| {
                    !is_critical_streaming_coord(task_coord, center)
                })
            else {
                work.state.requeue(coord);
                break;
            };
            work.state.defer_pending(preempted);
        }

        if work.generation_tasks.schedule(coord) {
            budget.record(1);
        } else {
            work.state.defer_pending(coord);
            budget.record(1);
        }
    }
}

fn dispatch_initial_mesh_tasks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
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

        // A stale or preempted initial mesh returns to `ready`. Rebuilding its
        // direct light on every retry overwrote light already converged by
        // background relaxation and invalidated the next snapshot yet again.
        // Seed and enqueue the initial relaxation only once per residency.
        if work.state.mark_initial_lighting_seeded(coord) {
            queues
                .fluid
                .enqueue_loaded_fluid_frontier(&work.world, coord);
            seed_chunk_direct_lighting(
                &mut work.world,
                coord,
                content.blocks(),
                content.fluids(),
                content.secondary_properties(),
            );
            if chunk_is_empty {
                queues.lighting.enqueue_empty_chunk_relaxation(coord);
            } else {
                queues.lighting.enqueue_chunk_relaxation(coord);
            }
        }

        if chunk_is_empty {
            integrate_empty_chunk(content, renderer, &work.world, &mut queues.remesh, coord);
            budget.record(1);
            continue;
        }

        let snapshot = ChunkMeshSnapshot::capture(&work.world, coord)
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
    notify_loaded_chunk_neighbors(coord, true, world, &renderer.pool, remesh_queue);
}

fn collect_built_chunk_meshes(
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
        if !work.state.keeps_loaded(completed.coord) || renderer.pool.contains(completed.coord) {
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
        let chunk_is_empty = chunk.is_empty();
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
        notify_loaded_chunk_neighbors(
            completed.coord,
            chunk_is_empty,
            &work.world,
            &renderer.pool,
            remesh_queue,
        );
    }
}

fn notify_loaded_chunk_neighbors(
    coord: IVec3,
    _chunk_is_empty: bool,
    world: &VoxelWorld,
    render_pool: &ChunkRenderPool,
    remesh_queue: &mut ChunkRemeshQueue,
) {
    let chunk = world
        .chunk(coord)
        .unwrap_or_else(|| panic!("rendered chunk data should exist at {coord:?}"));

    for offset in CARDINAL_NEIGHBORS {
        let neighbor = coord + offset;
        if !render_pool.contains(neighbor) {
            continue;
        }
        let Some(neighbor_chunk) = world.chunk(neighbor) else {
            continue;
        };

        // Geometry uses both neighbor occupancy and halo voxel lighting.
        // A newly rendered empty or air-boundary chunk can supply direct sky
        // light even when its relaxation makes no further voxel changes.
        // Refresh only rendered neighbors that actually have border content;
        // don't remesh every empty neighbor on each chunk integration.
        if neighbor_chunk.boundary_has_content(-offset) {
            remesh_queue.enqueue_priority(neighbor);
        }

        // Fluid surfaces are separate meshes and need their own refresh when
        // either side contributes boundary water. Geometry must not consume
        // their independent queued work.
        if chunk.boundary_has_fluid(offset) || neighbor_chunk.boundary_has_fluid(-offset) {
            remesh_queue.enqueue_fluid_priority(neighbor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_chunks_wait_inside_horizontal_retention_radius() {
        let near = IVec3::new(20, 0, 0);
        let far = IVec3::new(23, 0, 0);
        let mut state = ChunkStreamingState::default();
        state.enqueue_retired(near);
        state.enqueue_retired(far);

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 22),
            Some(far)
        );
        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::new(-3, 0, 0), 22),
            Some(near)
        );
    }

    #[test]
    fn retired_chunk_that_reenters_selection_is_not_unloaded() {
        let coord = IVec3::new(30, 0, 0);
        let mut state = ChunkStreamingState::default();
        state.enqueue_retired(coord);
        state.desired.insert(coord);

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 22),
            None
        );

        state.desired.remove(&coord);
        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 22),
            Some(coord)
        );
    }

    #[test]
    fn initial_lighting_seed_is_once_per_residency_not_per_mesh_retry() {
        let coord = IVec3::new(3, 1, -2);
        let mut state = ChunkStreamingState::default();

        assert!(state.mark_initial_lighting_seeded(coord));
        assert!(!state.mark_initial_lighting_seeded(coord));
        state.forget_initial_lighting_seeded(coord);
        assert!(state.mark_initial_lighting_seeded(coord));
    }
}
