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
        lighting::PendingLightingUpdates,
        mesh_snapshot::ChunkMeshSnapshot,
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
    tick::WorldTickClock,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CriticalPendingScanKey {
    queue_revision: u64,
    center: IVec3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RetiredScanKey {
    queue_revision: u64,
    selection_revision: u64,
    center: IVec2,
    radius_squared: i32,
}

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
    initial_mesh_seed_catchup: HashSet<IVec3>,
    selection_revision: u64,
    pending_critical_scan_miss: Option<CriticalPendingScanKey>,
    retired_scan_miss: Option<RetiredScanKey>,
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
        let scan_key = RetiredScanKey {
            queue_revision: self.retired.revision(),
            selection_revision: self.selection_revision,
            center,
            radius_squared,
        };
        if self.retired_scan_miss == Some(scan_key) {
            return None;
        }

        let desired = &self.desired;
        let retained = &self.retained;
        let coord = self.retired.pop_where(|coord| {
            if desired.contains(&coord) || retained.contains(&coord) {
                return false;
            }

            let delta = coord.xz() - center;
            delta.length_squared() > radius_squared
        });
        if coord.is_some() {
            self.retired_scan_miss = None;
        } else {
            self.retired_scan_miss = Some(scan_key);
        }
        coord
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
        let scan_key = CriticalPendingScanKey {
            queue_revision: self.pending.revision(),
            center,
        };
        if self.pending_critical_scan_miss == Some(scan_key) {
            return None;
        }

        let coord = self
            .pending
            .pop_where(|coord| is_critical_streaming_coord(coord, center));
        if coord.is_some() {
            self.pending_critical_scan_miss = None;
        } else {
            self.pending_critical_scan_miss = Some(scan_key);
        }
        coord
    }

    fn mark_ready(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.ready.contains(coord) {
            self.ready.enqueue(coord);
        }
    }

    fn pop_ready(&mut self) -> Option<IVec3> {
        let center = self.center?;
        let movement_direction = self.movement_direction;
        self.ready.pop_min_by_key(|coord| {
            if is_critical_streaming_coord(coord, center) {
                0_u8
            } else if movement_direction != IVec2::ZERO
                && (coord.xz() - center.xz()).dot(movement_direction) > 0
            {
                1
            } else {
                2
            }
        })
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
        self.initial_mesh_seed_catchup.remove(&coord);
    }


    fn mark_selection_rebuilt(&mut self) {
        self.selection_revision = self
            .selection_revision
            .checked_add(1)
            .expect("chunk streaming selection revision exhausted");
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
    world_ticks: Res<'w, WorldTickClock>,
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
    let current_tick = work.world_ticks.current_tick();

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
        collect_generated_chunks(&content, &mut work, &mut queues, current_tick);
    }
    if work.mesh_tasks.pending_count() > 0 {
        collect_built_chunk_meshes(&content, &mut renderer, &mut work, &mut queues.remesh);
    }
    if work.state.ready.len() > 0 {
        dispatch_initial_mesh_tasks(
            &content,
            &mut renderer,
            &mut work,
            &mut queues,
            current_tick,
        );
    }
    if work.state.pending.len() > 0 {
        dispatch_generation_tasks(
            &content,
            &renderer.pool,
            &mut work,
            &mut queues,
            current_tick,
        );
    }
}

// A resident chunk must never expose unseeded DARK light to a neighboring
// mesh snapshot. This used to happen while generated chunks waited in `ready`
// for a free mesh task slot, darkening whole faces during streaming. Preserve
// the once-per-residency rule and defer convergence through the existing queue.
fn seed_loaded_chunk_lighting(
    coord: IVec3,
    content: &ChunkContent<'_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
    if !work.state.mark_initial_lighting_seeded(coord) {
        return;
    }

    let chunk_is_empty = work
        .world
        .chunk(coord)
        .unwrap_or_else(|| panic!("seeded chunk must be resident: {coord:?}"))
        .is_empty();
    queues.fluid.reactivate_loaded_chunk(coord, current_tick);
    queues.fluid.enqueue_loaded_fluid_frontier(&work.world, coord);
    queues.lighting.seed_chunk_direct_lighting(
        &mut work.world,
        coord,
        content.blocks(),
        content.fluids(),
        content.secondary_properties(),
    );

    // A previously scheduled mesh may have captured a missing halo before
    // this chunk arrived. Reconcile after first publication, never cancel it.
    for y in -1..=1 {
        for z in -1..=1 {
            for x in -1..=1 {
                let offset = IVec3::new(x, y, z);
                if offset == IVec3::ZERO {
                    continue;
                }
                let neighbor = coord + offset;
                if work.mesh_tasks.contains(neighbor) {
                    work.state.initial_mesh_seed_catchup.insert(neighbor);
                }
            }
        }
    }

    if chunk_is_empty {
        queues.lighting.enqueue_empty_chunk_relaxation(coord);
    } else {
        queues.lighting.enqueue_chunk_relaxation(coord);
    }

    // Loading any 16³ section can change direct skylight for every resident
    // section below it in the same x/z column. Minecraft's light engine tracks
    // this through section/column status; Asteria explicitly invalidates the
    // lower resident sections so they converge against the new occluder.
    queues
        .lighting
        .enqueue_loaded_column_below(&work.world, coord);
}

fn collect_generated_chunks(
    content: &ChunkContent<'_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
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
        if work.world.has_resident_or_persisted_chunk(completed.coord) {
            if work.world.chunk(completed.coord).is_none() {
                assert!(
                    work.world.restore_chunk(completed.coord),
                    "resident or persisted chunk must remain resident or archived: {:?}",
                    completed.coord
                );
            }
            seed_loaded_chunk_lighting(completed.coord, content, work, queues, current_tick);
            work.state.mark_ready(completed.coord);
            continue;
        }

        work.world.insert_chunk(completed.coord, completed.output);
        seed_loaded_chunk_lighting(completed.coord, content, work, queues, current_tick);
        work.state.mark_ready(completed.coord);
    }
}

fn dispatch_generation_tasks(
    content: &ChunkContent<'_>,
    render_pool: &ChunkRenderPool,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
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

        if work.world.has_resident_or_persisted_chunk(coord) {
            assert!(
                work.world.restore_chunk(coord),
                "resident or persisted chunk must remain resident or archived: {coord:?}"
            );
            seed_loaded_chunk_lighting(coord, content, work, queues, current_tick);
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

        // Also covers a resident chunk that reached `ready` by a path other
        // than generated-result integration. No mesh may capture it as DARK.
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
    notify_loaded_chunk_neighbors(coord, world, &renderer.pool, remesh_queue);
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
        let chunk_has_fluid = chunk.has_fluid();
        let catchup = completed.output.dependencies.needs_initial_catchup(&work.world)
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

    // Face lighting, AO and fluid corner heights sample edge/corner neighbors
    // as well as cardinals. A chunk that arrives already seeded can change
    // the halo without causing any later relaxation changes. Notify only
    // already-rendered neighbors whose toward-source boundary has content.
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
    fn ready_queue_preserves_critical_forward_background_priority_in_one_scan() {
        let background = IVec3::new(-4, 0, 0);
        let forward = IVec3::new(5, 0, 0);
        let critical = IVec3::new(1, 0, 0);
        let mut state = ChunkStreamingState {
            center: Some(IVec3::ZERO),
            movement_direction: IVec2::X,
            ..default()
        };
        state.ready.enqueue(background);
        state.ready.enqueue(forward);
        state.ready.enqueue(critical);

        assert_eq!(state.pop_ready(), Some(critical));
        assert_eq!(state.pop_ready(), Some(forward));
        assert_eq!(state.pop_ready(), Some(background));
        assert_eq!(state.pop_ready(), None);
    }

    #[test]
    fn critical_pending_scan_miss_retries_only_after_queue_or_center_change() {
        let far = IVec3::new(8, 0, 0);
        let critical = IVec3::new(1, 0, 0);
        let mut state = ChunkStreamingState {
            center: Some(IVec3::ZERO),
            ..default()
        };
        state.pending.enqueue(far);

        assert_eq!(state.pop_critical_pending(), None);
        let first_miss = state.pending_critical_scan_miss;
        assert!(first_miss.is_some());

        assert_eq!(state.pop_critical_pending(), None);
        assert_eq!(state.pending_critical_scan_miss, first_miss);

        state.pending.enqueue(critical);
        assert_eq!(state.pop_critical_pending(), Some(critical));

        state.center = Some(IVec3::new(8, 0, 0));
        assert_eq!(state.pop_critical_pending(), Some(far));
    }

    #[test]
    fn retired_scan_miss_invalidates_when_selection_rebuilds() {
        let coord = IVec3::new(20, 0, 0);
        let mut state = ChunkStreamingState::default();
        state.enqueue_retired(coord);
        state.retained.insert(coord);

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 10),
            None
        );
        let first_miss = state.retired_scan_miss;
        assert!(first_miss.is_some());

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 10),
            None
        );
        assert_eq!(state.retired_scan_miss, first_miss);

        state.retained.remove(&coord);
        state.mark_selection_rebuilt();

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 10),
            Some(coord)
        );
    }

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
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 22),
            None
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
        state.initial_mesh_seed_catchup.insert(coord);
        state.forget_initial_lighting_seeded(coord);
        assert!(state.mark_initial_lighting_seeded(coord));
        assert!(!state.initial_mesh_seed_catchup.contains(&coord));
    }

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
