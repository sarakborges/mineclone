mod generation;
mod generation_wave;
mod initial_presentation;
mod mesh_pressure;
mod meshing;
mod pending;
mod priority_diagnostics;
mod ready;
mod residency;
mod selection;
mod selection_state;
mod surface_cache;

use std::time::{Duration, Instant};

use bevy::{
    ecs::system::SystemParam,
    platform::collections::HashSet,
    prelude::*,
};

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::chunk_coord_from_position,
        deduplicated_queue::DeduplicatedQueue,
        lighting::{DirectLightingSeedResult, PendingLightingUpdates},
        meshlet::ChunkMeshletMask,
        world::VoxelWorld,
    },
};

use self::{
    generation::{collect_generated_chunks, dispatch_generation_tasks},
    generation_wave::GenerationWaveState,
    initial_presentation::InitialPresentationState,
    mesh_pressure::MeshPressureState,
    meshing::{collect_built_chunk_meshes, dispatch_initial_mesh_tasks},
    pending::PendingChunkQueue,
    priority_diagnostics::{StreamingPriorityDiagnostics, StreamingPriorityScanDiagnostic},
    ready::ReadyChunkQueue,
    residency::ChunkResidencyState,
    selection::rebuild_queue,
    selection_state::StreamingSelectionState,
    surface_cache::StreamingSelectionCache,
};
pub(in crate::world) use self::{
    generation::refill_generation_workers,
    selection::initial_streaming_chunk_coords,
};
use super::{
    biome_field::BiomeField,
    chunk_async_work::ChunkAsyncWorkLimiter,
    chunk_generation_tasks::ChunkGenerationTasks,
    chunk_mesh_tasks::ChunkMeshTasks,
    chunk_remesh::ChunkRemeshQueue,
    chunk_rendering::ChunkRenderPool,
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    fluid_updates::PendingFluidUpdates,
    render_distance::{RenderDistanceSettings, chunk_visibility_radii},
    tick::WorldTickClock,
    work_budget::WorldFrameWorkBudget,
    warp::PendingWarp,
    world_feature_fields::WorldFeatureFields,
};

const CRITICAL_PLAYER_RADIUS_CHUNKS: i32 = 1;
const SLOW_STREAMING_REBUILD_WARNING: Duration = Duration::from_millis(8);

pub(super) type ChunkLoadPriority = (i64, i64, i32, i32, i32, i32);

#[derive(Resource, Default)]
pub(super) struct ChunkStreamingState {
    selection_state: StreamingSelectionState,
    residency: ChunkResidencyState,
    pending: PendingChunkQueue,
    ready: ReadyChunkQueue,
    selection_cache: StreamingSelectionCache,
    initial_presentation: InitialPresentationState,
    mesh_pressure: MeshPressureState,
    generation_wave: GenerationWaveState,
    priority_diagnostics: StreamingPriorityDiagnostics,
}

impl ChunkStreamingState {
    pub(super) fn center(&self) -> Option<IVec3> {
        self.selection_state.center()
    }

    pub(super) fn movement_direction(&self) -> IVec2 {
        self.selection_state.movement_direction()
    }

    pub(super) fn selection_revision(&self) -> u64 {
        self.residency.revision()
    }

    pub(super) fn keeps_loaded(&self, coord: IVec3) -> bool {
        self.residency.keeps_loaded(coord)
    }

    pub(super) fn retains_render_mesh(&self, coord: IVec3) -> bool {
        let Some(center) = self.selection_state.center() else {
            return false;
        };
        let (_, hide_radius) =
            chunk_visibility_radii(self.selection_state.horizontal_radius());
        self.keeps_loaded(coord) && chunk_is_inside_render_radius(center, coord, hide_radius)
    }

    pub(super) fn enqueue_retired(&mut self, coord: IVec3) {
        self.residency.enqueue_retired(coord);
    }

    pub(super) fn pop_retired_outside_horizontal_radius(
        &mut self,
        center: IVec3,
        horizontal_radius: i32,
    ) -> Option<IVec3> {
        self.residency
            .pop_retired_outside_horizontal_radius(center, horizontal_radius)
    }

    fn requeue(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.pending.contains(coord) && !self.ready.contains(coord) {
            self.pending.enqueue_front(coord);
        }
    }

    fn has_critical_pending(&mut self) -> bool {
        let Some(center) = self.selection_state.center() else {
            return false;
        };
        self.pending
            .has_critical(center, |coord| is_critical_streaming_coord(coord, center))
    }

    fn pop_pending_by_priority(&mut self) -> Option<IVec3> {
        let center = self.selection_state.center()?;
        let selection_revision = self.residency.revision();
        let movement_direction = self.selection_state.movement_direction();
        let visible_radius = self.selection_state.horizontal_radius();
        let structure_top_chunks = self.selection_cache.structure_top_chunks();
        let center_structure_top_chunk = structure_top_chunks
            .get(&center.xz())
            .copied()
            .unwrap_or(0);
        let surface_ranges = self.selection_cache.surface_ranges();
        let prioritize_surface = selection::player_is_above_surface(
            center,
            center_structure_top_chunk,
            surface_ranges,
        );

        let (selected, scan) = self.pending.pop_by_priority(selection_revision, |coord| {
            (
                selection::pending_priority(
                    coord,
                    center,
                    visible_radius,
                    structure_top_chunks
                        .get(&coord.xz())
                        .copied()
                        .unwrap_or(0),
                    movement_direction,
                    prioritize_surface,
                    surface_ranges,
                ),
                coord.y,
                coord.z,
                coord.x,
            )
        });
        self.priority_diagnostics.record_pending(scan);
        selected
    }

    fn generation_dispatch_work_exists(&self) -> bool {
        self.generation_wave.dispatch_work_exists(self.pending.len())
    }

    pub(in crate::world) fn generated_chunk_is_unpublished(&self, coord: IVec3) -> bool {
        self.generation_wave.contains_unpublished(coord)
    }

    pub(in crate::world) fn generated_fluid_settling_owns_mutation(&self, coord: IVec3) -> bool {
        self.generation_wave.fluid_settling.owns_mutation(coord)
    }

    fn resident_generated_chunk_is_unpublished(&self, coord: IVec3) -> bool {
        self.generation_wave
            .resident_generated_chunk_is_unpublished(coord)
    }

    fn adopt_structure_top_chunk(&mut self, horizontal: IVec2, top_chunk: i32) {
        let Some((previous_top, surface_top_chunk)) = self
            .selection_cache
            .adopt_structure_top_chunk(horizontal, top_chunk)
        else {
            return;
        };
        let start_y = previous_top.max(surface_top_chunk).saturating_add(1).max(0);
        let mut changed = false;
        for y in start_y..=top_chunk {
            let coord = IVec3::new(horizontal.x, y, horizontal.y);
            if !self.residency.desired.insert(coord) {
                continue;
            }
            changed = true;
            if !self.pending.contains(coord)
                && !self.ready.contains(coord)
                && !self.generated_chunk_is_unpublished(coord)
                && !self.mesh_is_pressure_evicted(coord)
            {
                self.pending.enqueue(coord);
            }
        }
        if changed {
            self.mark_selection_rebuilt();
        }
    }

    fn mark_ready(&mut self, coord: IVec3) {
        assert!(
            !self.resident_generated_chunk_is_unpublished(coord),
            "generated chunk cannot become ready before fluid settling completes: {coord:?}"
        );
        if self.mesh_pressure.contains(coord) {
            return;
        }
        if self.keeps_loaded(coord) && !self.ready.contains(coord) {
            self.ready.enqueue(coord);
        }
    }

    pub(super) fn suppress_mesh_for_pressure(&mut self, coord: IVec3, bytes: usize) {
        self.ready.remove(coord);
        self.mesh_pressure.suppress(coord, bytes);
    }

    pub(super) fn recover_mesh_after_pressure(&mut self, coord: IVec3) -> bool {
        if !self.mesh_pressure.recover(coord) || !self.keeps_loaded(coord) {
            return false;
        }
        self.mark_ready(coord);
        true
    }

    pub(super) fn mesh_pressure_evicted_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.mesh_pressure.coords()
    }

    pub(super) fn mesh_pressure_evicted_bytes(&self, coord: IVec3) -> Option<usize> {
        self.mesh_pressure.bytes(coord)
    }

    pub(super) fn mesh_is_pressure_evicted(&self, coord: IVec3) -> bool {
        self.mesh_pressure.contains(coord)
    }

    pub(super) fn retain_mesh_pressure_evictions(
        &mut self,
        desired: &HashSet<IVec3>,
        center: IVec3,
    ) {
        self.mesh_pressure
            .retain_for_selection(desired, |coord| !is_critical_streaming_coord(coord, center));
    }

    fn pop_ready(&mut self) -> Option<IVec3> {
        let center = self.selection_state.center()?;
        let movement_direction = self.selection_state.movement_direction();
        let (show_radius, _) =
            chunk_visibility_radii(self.selection_state.horizontal_radius());
        let radius = i64::from(show_radius.max(0));
        let radius_squared = radius * radius;
        let selection_revision = self.residency.revision();
        let desired = &self.residency.desired;
        let retained = &self.residency.retained;
        let (selected, scan) = self.ready.pop_min_where_by_key(
            selection_revision,
            center.xz(),
            radius_squared,
            |coord| {
                (desired.contains(&coord) || retained.contains(&coord))
                    && chunk_is_inside_render_radius(center, coord, show_radius)
            },
            |coord| chunk_load_priority(coord, center, movement_direction),
        );
        self.priority_diagnostics.record_ready(scan);
        selected
    }

    fn defer_ready(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.ready.contains(coord) {
            self.ready.enqueue_front(coord);
        }
    }

    fn mark_initial_lighting_seeded(&mut self, coord: IVec3) -> bool {
        self.initial_presentation.mark_lighting_seeded(coord)
    }

    fn store_initial_lighting_seed_result(
        &mut self,
        coord: IVec3,
        result: DirectLightingSeedResult,
    ) {
        self.initial_presentation
            .store_lighting_seed_result(coord, result);
    }

    fn take_initial_lighting_seed_result(
        &mut self,
        coord: IVec3,
    ) -> Option<DirectLightingSeedResult> {
        self.initial_presentation.take_lighting_seed_result(coord)
    }

    fn mark_initial_lighting_activated(&mut self, coord: IVec3) -> bool {
        self.initial_presentation.mark_lighting_activated(coord)
    }

    fn add_initial_mesh_seed_catchup(&mut self, coord: IVec3, meshlets: ChunkMeshletMask) {
        self.initial_presentation
            .add_mesh_seed_catchup(coord, meshlets);
    }

    pub(super) fn initial_mesh_seed_catchup(&self, coord: IVec3) -> Option<ChunkMeshletMask> {
        self.initial_presentation.mesh_seed_catchup(coord)
    }

    pub(super) fn clear_initial_mesh_seed_catchup(&mut self, coord: IVec3) {
        self.initial_presentation.clear_mesh_seed_catchup(coord);
    }

    pub(super) fn forget_initial_lighting_seeded(&mut self, coord: IVec3) {
        self.initial_presentation.forget(coord);
    }

    pub(super) fn take_priority_scan_diagnostics(
        &self,
    ) -> (
        StreamingPriorityScanDiagnostic,
        StreamingPriorityScanDiagnostic,
    ) {
        self.priority_diagnostics.take()
    }

    pub(crate) fn has_renderable_streaming_backlog(&self) -> bool {
        let Some(center) = self.selection_state.center() else {
            return false;
        };
        let (show_radius, _) =
            chunk_visibility_radii(self.selection_state.horizontal_radius());
        let renderable = |coord: IVec3| {
            self.keeps_loaded(coord)
                && chunk_is_inside_render_radius(center, coord, show_radius)
        };

        self.ready.values().any(renderable)
            || self.pending.values().any(renderable)
            || self.generation_wave.targets().any(renderable)
    }

    pub(super) fn diagnostic_counts(&self) -> (usize, usize, usize, usize, usize, usize) {
        let (generation_pending, generation_targets, staged_generated) =
            self.generation_wave.diagnostic_counts();
        (
            self.pending.len(),
            self.ready.len(),
            generation_pending,
            generation_targets,
            staged_generated,
            self.mesh_pressure.len(),
        )
    }

    pub(super) fn diagnostic_generation_prefetch_count(&self) -> usize {
        self.generation_wave.prefetch_count()
    }

    pub(super) fn diagnostic_fluid_settling_counts(
        &self,
    ) -> (bool, usize, usize, usize, usize, usize, usize) {
        self.generation_wave.fluid_settling.diagnostic_counts()
    }

    pub(super) fn diagnostic_renderable_backlog_counts(&self) -> (usize, usize, usize) {
        let Some(center) = self.selection_state.center() else {
            return (0, 0, 0);
        };
        let (show_radius, _) =
            chunk_visibility_radii(self.selection_state.horizontal_radius());
        let renderable = |coord: IVec3| {
            self.keeps_loaded(coord)
                && chunk_is_inside_render_radius(center, coord, show_radius)
        };

        (
            self.pending.values().filter(|coord| renderable(*coord)).count(),
            self.ready.values().filter(|coord| renderable(*coord)).count(),
            self.generation_wave
                .targets()
                .filter(|coord| renderable(*coord))
                .count(),
        )
    }

    fn mark_selection_rebuilt(&mut self) {
        self.residency.mark_rebuilt();
        let residency = &self.residency;
        self.ready.retain(|coord| residency.keeps_loaded(coord));
    }
}

pub(super) fn chunk_load_priority(
    coord: IVec3,
    center: IVec3,
    movement_direction: IVec2,
) -> ChunkLoadPriority {
    let dx = i64::from(coord.x) - i64::from(center.x);
    let dy = i64::from(coord.y) - i64::from(center.y);
    let dz = i64::from(coord.z) - i64::from(center.z);
    let horizontal_distance = dx * dx + dz * dz;
    let total_distance = horizontal_distance + dy * dy;
    let forward = dx * i64::from(movement_direction.x)
        + dz * i64::from(movement_direction.y);
    let directional_band = if movement_direction == IVec2::ZERO || forward == 0 {
        1
    } else if forward > 0 {
        0
    } else {
        2
    };

    (
        horizontal_distance,
        total_distance,
        directional_band,
        coord.y,
        coord.z,
        coord.x,
    )
}

fn is_critical_streaming_coord(coord: IVec3, center: IVec3) -> bool {
    let delta = coord - center;
    delta.x.abs() <= CRITICAL_PLAYER_RADIUS_CHUNKS
        && delta.y.abs() <= CRITICAL_PLAYER_RADIUS_CHUNKS
        && delta.z.abs() <= CRITICAL_PLAYER_RADIUS_CHUNKS
}

fn chunk_is_inside_render_radius(center: IVec3, coord: IVec3, horizontal_radius: i32) -> bool {
    if horizontal_radius < 0 {
        return false;
    }

    let delta_x = i64::from(coord.x) - i64::from(center.x);
    let delta_z = i64::from(coord.z) - i64::from(center.z);
    let radius = i64::from(horizontal_radius);
    delta_x * delta_x + delta_z * delta_z <= radius * radius
}

struct QueueRebuildContext<'a> {
    render_pool: &'a ChunkRenderPool,
    dimension: &'a DimensionDefinition,
    biomes: &'a BiomeRegistry,
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
    frame_budget: Res<'w, WorldFrameWorkBudget>,
    async_work: Res<'w, ChunkAsyncWorkLimiter>,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingSelection<'w, 's> {
    render_distance: Res<'w, RenderDistanceSettings>,
    pending_warp: Res<'w, PendingWarp>,
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
    let warp_center = selection.pending_warp.streaming_center();
    let player_chunk =
        warp_center.unwrap_or_else(|| chunk_coord_from_position(feet_position));
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let (horizontal_radius, vertical_radius) = selection
        .pending_warp
        .streaming_radii()
        .unwrap_or_else(|| {
            (
                selection.render_distance.chunks(),
                selection.render_distance.vertical_chunks(),
            )
        });
    let allow_forward_preload = warp_center.is_none();
    let current_tick = work.world_ticks.current_tick();

    if work
        .state
        .selection_state
        .needs_rebuild(center, horizontal_radius, vertical_radius)
    {
        let rebuild_context = QueueRebuildContext {
            render_pool: &renderer.pool,
            dimension: generation.dimension(),
            biomes: &content.biomes,
            biome_field: &content.biome_field,
            feature_fields: &generation.feature_fields,
        };
        let rebuild_started = Instant::now();
        rebuild_queue(
            &mut work.state,
            center,
            horizontal_radius,
            vertical_radius,
            allow_forward_preload,
            &mut selection.scratch,
            &rebuild_context,
        );
        let rebuild_elapsed = rebuild_started.elapsed();
        if rebuild_elapsed >= SLOW_STREAMING_REBUILD_WARNING {
            warn!(
                "slow streaming selection rebuild: center={center:?} radius={horizontal_radius} vertical_radius={vertical_radius} warp={} desired={} pending={} structure_columns={} elapsed_ms={:.2}",
                !allow_forward_preload,
                work.state.residency.desired.len(),
                work.state.pending.len(),
                work.state.selection_cache.structure_column_count(),
                rebuild_elapsed.as_secs_f64() * 1_000.0,
            );
        }

        let cancelled_generation = {
            let state = &work.state;
            work.generation_tasks
                .cancel_where(|coord| !state.keeps_loaded(coord))
        };
        for coord in cancelled_generation {
            work.state.generation_wave.abandon_target(coord);
        }

        let cancelled_meshes = {
            let state = &work.state;
            work.mesh_tasks
                .cancel_where(|coord| !state.retains_render_mesh(coord))
        };
        for coord in cancelled_meshes {
            work.state.clear_initial_mesh_seed_catchup(coord);
        }
    }

    work.generation_tasks.sync_snapshot(&generation, &content);
    work.generation_tasks.sync_streaming_region(center);
    work.mesh_tasks.sync_snapshot(&content);

    if work.mesh_tasks.pending_count() > 0 {
        collect_built_chunk_meshes(
            &content,
            &mut renderer,
            &mut work,
            &mut queues,
            current_tick,
        );
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
    if work.generation_tasks.pending_count() > 0 || work.state.generation_wave.is_active() {
        collect_generated_chunks(&content, &mut work, &mut queues, current_tick);
    }
    if work.state.generation_dispatch_work_exists() {
        dispatch_generation_tasks(&renderer.pool, &mut work);
    }
}

pub(super) fn seed_loaded_chunk_direct_lighting(
    coord: IVec3,
    content: &ChunkContent<'_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
) {
    if !work.state.mark_initial_lighting_seeded(coord) {
        return;
    }

    let lighting_seed = queues.lighting.seed_chunk_direct_lighting(
        &mut work.world,
        coord,
        content.blocks(),
        content.fluids(),
        content.secondary_properties(),
    );
    work.state
        .store_initial_lighting_seed_result(coord, lighting_seed);

    for y in -1..=1 {
        for z in -1..=1 {
            for x in -1..=1 {
                let offset = IVec3::new(x, y, z);
                if offset == IVec3::ZERO {
                    continue;
                }
                let neighbor = coord + offset;
                if work.mesh_tasks.contains(neighbor) {
                    let meshlets = ChunkMeshletMask::for_dependency_offset(-offset);
                    work.state.add_initial_mesh_seed_catchup(neighbor, meshlets);
                }
            }
        }
    }
}

pub(super) fn activate_published_chunk_runtime(
    coord: IVec3,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
    if !work.state.mark_initial_lighting_activated(coord) {
        return;
    }

    let lighting_seed = work
        .state
        .take_initial_lighting_seed_result(coord)
        .expect("newly activated chunk must retain its direct-light seed result");
    let chunk_is_empty = work
        .world
        .chunk(coord)
        .unwrap_or_else(|| panic!("activated chunk must be resident: {coord:?}"))
        .is_empty();

    queues.fluid.reactivate_loaded_chunk(coord, current_tick);
    queues.fluid.enqueue_loaded_fluid_frontier(&work.world, coord);

    if chunk_is_empty {
        queues.lighting.enqueue_empty_chunk_relaxation(coord);
    } else if lighting_seed.requires_relaxation {
        queues.lighting.enqueue_chunk_relaxation(coord);
    }

    if lighting_seed.changes_direct_sky_below {
        queues
            .lighting
            .enqueue_loaded_column_below(&work.world, coord);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_selection(
        center: Option<IVec3>,
        movement_direction: IVec2,
        horizontal_radius: i32,
    ) -> ChunkStreamingState {
        ChunkStreamingState {
            selection_state: StreamingSelectionState::configured(
                center,
                movement_direction,
                horizontal_radius,
                0,
            ),
            ..default()
        }
    }

    #[test]
    fn ready_queue_prioritizes_distance_before_movement_direction() {
        let background = IVec3::new(-4, 0, 0);
        let forward = IVec3::new(5, 0, 0);
        let critical = IVec3::new(1, 0, 0);
        let mut state = state_with_selection(Some(IVec3::ZERO), IVec2::X, 12);
        state.residency.desired.extend([background, forward, critical]);
        state.ready.enqueue(background);
        state.ready.enqueue(forward);
        state.ready.enqueue(critical);

        assert_eq!(state.pop_ready(), Some(critical));
        assert_eq!(state.pop_ready(), Some(background));
        assert_eq!(state.pop_ready(), Some(forward));
        assert_eq!(state.pop_ready(), None);
    }

    #[test]
    fn retired_scan_miss_invalidates_when_selection_rebuilds() {
        let coord = IVec3::new(20, 0, 0);
        let mut state = ChunkStreamingState::default();
        state.enqueue_retired(coord);
        state.residency.retained.insert(coord);

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 10),
            None
        );
        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 10),
            None
        );

        state.residency.retained.remove(&coord);
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
        state.residency.desired.insert(coord);

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 22),
            None
        );

        state.residency.desired.remove(&coord);
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
        state.add_initial_mesh_seed_catchup(coord, ChunkMeshletMask::ALL);
        state.forget_initial_lighting_seeded(coord);
        assert!(state.mark_initial_lighting_seeded(coord));
        assert!(state.initial_mesh_seed_catchup(coord).is_none());
    }

    #[test]
    fn forward_preload_stays_resident_without_allocating_a_gpu_mesh() {
        let visible = IVec3::new(14, 0, 0);
        let hysteresis = IVec3::new(15, 0, 0);
        let preload_only = IVec3::new(20, 0, 0);
        let mut state = state_with_selection(Some(IVec3::ZERO), IVec2::ZERO, 12);
        state
            .residency
            .desired
            .extend([visible, hysteresis, preload_only]);

        assert!(state.retains_render_mesh(hysteresis));
        assert!(!state.retains_render_mesh(preload_only));

        state.mark_ready(preload_only);
        state.mark_ready(hysteresis);
        state.mark_ready(visible);
        assert_eq!(state.pop_ready(), Some(visible));
        assert!(state.ready.contains(hysteresis));
        assert!(state.ready.contains(preload_only));
    }

    #[test]
    fn ready_scan_miss_invalidates_when_selection_or_queue_changes() {
        let preload_only = IVec3::new(20, 0, 0);
        let mut state = state_with_selection(Some(IVec3::ZERO), IVec2::ZERO, 12);
        state.residency.desired.insert(preload_only);
        state.mark_ready(preload_only);

        assert_eq!(state.pop_ready(), None);
        assert_eq!(state.pop_ready(), None);

        let visible = IVec3::X;
        state.residency.desired.insert(visible);
        state.mark_ready(visible);
        assert_eq!(state.pop_ready(), Some(visible));

        assert_eq!(state.pop_ready(), None);
        state.selection_state.commit_rebuild(IVec3::new(6, 0, 0), 12, 0);
        state.mark_selection_rebuilt();
        assert_eq!(state.pop_ready(), Some(preload_only));
    }

    #[test]
    fn pending_priority_cache_survives_its_own_queue_pops() {
        let near = IVec3::X;
        let middle = IVec3::new(2, 0, 0);
        let far = IVec3::new(3, 0, 0);
        let mut state = state_with_selection(Some(IVec3::ZERO), IVec2::ZERO, 12);
        state.residency.desired.extend([near, middle, far]);
        state.pending.enqueue(far);
        state.pending.enqueue(near);
        state.pending.enqueue(middle);

        assert_eq!(state.pop_pending_by_priority(), Some(near));
        assert_eq!(state.pop_pending_by_priority(), Some(middle));
        assert_eq!(state.pop_pending_by_priority(), Some(far));
        assert_eq!(state.pop_pending_by_priority(), None);
    }

    #[test]
    fn critical_pending_scan_miss_invalidates_when_queue_or_center_changes() {
        let far = IVec3::new(10, 0, 0);
        let mut state = state_with_selection(Some(IVec3::ZERO), IVec2::ZERO, 0);
        state.pending.enqueue(far);

        assert!(!state.has_critical_pending());
        assert!(!state.has_critical_pending());

        state.pending.enqueue(IVec3::X);
        assert!(state.has_critical_pending());

        state.pending.remove(IVec3::X);
        assert!(!state.has_critical_pending());
        state.selection_state.commit_rebuild(IVec3::new(9, 0, 0), 0, 0);
        assert!(state.has_critical_pending());
    }

    #[test]
    fn renderable_streaming_backlog_includes_generation_work() {
        let visible = IVec3::new(3, 0, 0);
        let preload_only = IVec3::new(20, 0, 0);
        let mut state = state_with_selection(Some(IVec3::ZERO), IVec2::ZERO, 12);
        state.residency.desired.extend([visible, preload_only]);

        state.pending.enqueue(preload_only);
        assert!(!state.has_renderable_streaming_backlog());

        state.pending.enqueue(visible);
        assert!(state.has_renderable_streaming_backlog());
    }

    #[test]
    fn prefetched_generation_is_reserved_and_promoted_to_next_wave() {
        let coord = IVec3::new(4, 0, -2);
        let mut state = ChunkStreamingState::default();

        state.generation_wave.mark_prefetched(coord);
        assert!(state.generated_chunk_is_unpublished(coord));
        assert_eq!(state.diagnostic_generation_prefetch_count(), 1);
        assert!(!state.generation_wave.contains_target(coord));

        state.generation_wave.finish();

        assert_eq!(state.diagnostic_generation_prefetch_count(), 0);
        assert!(state.generation_wave.contains_target(coord));
        assert!(state.generated_chunk_is_unpublished(coord));
    }

    #[test]
    fn abandoning_generation_removes_prefetch_reservation() {
        let coord = IVec3::new(-5, 1, 7);
        let mut state = ChunkStreamingState::default();

        state.generation_wave.mark_prefetched(coord);
        state.generation_wave.abandon_target(coord);

        assert_eq!(state.diagnostic_generation_prefetch_count(), 0);
        assert!(!state.generated_chunk_is_unpublished(coord));
    }
}
