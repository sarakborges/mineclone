mod generation;
mod meshing;
mod selection;
mod surface_cache;

use std::{
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use bevy::{
    ecs::system::SystemParam,
    platform::collections::{HashMap, HashSet},
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
    meshing::{collect_built_chunk_meshes, dispatch_initial_mesh_tasks},
    selection::rebuild_queue,
};
pub(in crate::world) use self::selection::initial_streaming_chunk_coords;
use super::{
    biome_field::BiomeField,
    chunk_async_work::ChunkAsyncWorkLimiter,
    chunk_generation_tasks::ChunkGenerationTasks,
    chunk_mesh_tasks::ChunkMeshTasks,
    chunk_remesh::ChunkRemeshQueue,
    chunk_rendering::ChunkRenderPool,
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    fluid_updates::{GeneratedFluidSettling, PendingFluidUpdates},
    render_distance::{RenderDistanceSettings, chunk_visibility_radii},
    tick::WorldTickClock,
    work_budget::WorldFrameWorkBudget,
    warp::PendingWarp,
    world_feature_fields::WorldFeatureFields,
};

const CRITICAL_PLAYER_RADIUS_CHUNKS: i32 = 1;
const SLOW_STREAMING_REBUILD_WARNING: Duration = Duration::from_millis(8);

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct StreamingPriorityScanDiagnostic {
    pub(super) count: u64,
    pub(super) average_micros: u64,
    pub(super) max_micros: u64,
    pub(super) max_queue_len: usize,
}

#[derive(Default)]
struct StreamingPriorityScanMetrics {
    count: AtomicU64,
    total_nanos: AtomicU64,
    max_nanos: AtomicU64,
    max_queue_len: AtomicUsize,
}

impl StreamingPriorityScanMetrics {
    fn record(&self, elapsed: Duration, queue_len: usize) {
        let elapsed_nanos = elapsed.as_nanos().min(u128::from(u64::MAX)) as u64;
        self.count.fetch_add(1, Ordering::Relaxed);
        self.total_nanos
            .fetch_add(elapsed_nanos, Ordering::Relaxed);
        self.max_nanos.fetch_max(elapsed_nanos, Ordering::Relaxed);
        self.max_queue_len.fetch_max(queue_len, Ordering::Relaxed);
    }

    fn take(&self) -> StreamingPriorityScanDiagnostic {
        let count = self.count.swap(0, Ordering::Relaxed);
        let total_nanos = self.total_nanos.swap(0, Ordering::Relaxed);
        StreamingPriorityScanDiagnostic {
            count,
            average_micros: total_nanos.checked_div(count).unwrap_or(0) / 1_000,
            max_micros: self.max_nanos.swap(0, Ordering::Relaxed) / 1_000,
            max_queue_len: self.max_queue_len.swap(0, Ordering::Relaxed),
        }
    }
}

#[derive(Default)]
struct StreamingPriorityDiagnostics {
    pending: StreamingPriorityScanMetrics,
    ready: StreamingPriorityScanMetrics,
}

pub(super) type ChunkLoadPriority = (i64, i64, i32, i32, i32, i32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SelectionScanKey {
    queue_revision: u64,
    selection_revision: u64,
    center: IVec2,
    radius_squared: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CriticalPendingScanKey {
    queue_revision: u64,
    center: IVec3,
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
    surface_support_minimums: HashMap<IVec2, i32>,
    structure_top_chunks: HashMap<IVec2, i32>,
    initial_lighting_seeded: HashSet<IVec3>,
    initial_lighting_seed_results: HashMap<IVec3, DirectLightingSeedResult>,
    initial_lighting_activated: HashSet<IVec3>,
    initial_mesh_seed_catchup: HashMap<IVec3, ChunkMeshletMask>,
    mesh_pressure_evicted: HashMap<IVec3, usize>,
    fluid_settling: GeneratedFluidSettling,
    generation_wave_targets: HashSet<IVec3>,
    generation_wave_pending: DeduplicatedQueue<IVec3>,
    staged_generated_chunks: HashSet<IVec3>,
    settled_publication_chunks: Vec<IVec3>,
    selection_revision: u64,
    pending_critical_scan_miss: Option<CriticalPendingScanKey>,
    ready_scan_miss: Option<SelectionScanKey>,
    retired_scan_miss: Option<SelectionScanKey>,
    priority_diagnostics: StreamingPriorityDiagnostics,
}

impl ChunkStreamingState {
    pub(super) fn center(&self) -> Option<IVec3> {
        self.center
    }

    pub(super) fn movement_direction(&self) -> IVec2 {
        self.movement_direction
    }

    pub(super) fn selection_revision(&self) -> u64 {
        self.selection_revision
    }

    pub(super) fn keeps_loaded(&self, coord: IVec3) -> bool {
        self.desired.contains(&coord) || self.retained.contains(&coord)
    }

    pub(super) fn retains_render_mesh(&self, coord: IVec3) -> bool {
        let Some(center) = self.center else {
            return false;
        };
        let (_, hide_radius) = chunk_visibility_radii(self.horizontal_radius);
        self.keeps_loaded(coord) && chunk_is_inside_render_radius(center, coord, hide_radius)
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
        let radius = i64::from(horizontal_radius.max(0));
        let radius_squared = radius * radius;
        let scan_key = SelectionScanKey {
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

            let delta_x = i64::from(coord.x) - i64::from(center.x);
            let delta_z = i64::from(coord.z) - i64::from(center.y);
            delta_x * delta_x + delta_z * delta_z > radius_squared
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

    fn has_critical_pending(&mut self) -> bool {
        let Some(center) = self.center else {
            return false;
        };
        let scan_key = CriticalPendingScanKey {
            queue_revision: self.pending.revision(),
            center,
        };
        if self.pending_critical_scan_miss == Some(scan_key) {
            return false;
        }

        let found = self
            .pending
            .values()
            .any(|coord| is_critical_streaming_coord(coord, center));
        if found {
            self.pending_critical_scan_miss = None;
        } else {
            self.pending_critical_scan_miss = Some(scan_key);
        }
        found
    }

    fn pop_pending_by_priority(&mut self) -> Option<IVec3> {
        let center = self.center?;
        let movement_direction = self.movement_direction;
        let visible_radius = self.horizontal_radius;
        let center_structure_top_chunk = self
            .structure_top_chunks
            .get(&center.xz())
            .copied()
            .unwrap_or(0);
        let prioritize_surface = selection::player_is_above_surface(
            center,
            center_structure_top_chunk,
            &self.surface_ranges,
        );
        let structure_top_chunks = &self.structure_top_chunks;
        let surface_ranges = &self.surface_ranges;

        let queue_len = self.pending.len();
        let started = Instant::now();
        let selected = self.pending.pop_min_by_key(|coord| {
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
        self.priority_diagnostics
            .pending
            .record(started.elapsed(), queue_len);
        selected
    }

    fn start_generation_wave_target(&mut self, coord: IVec3) {
        if self.generation_wave_targets.insert(coord) {
            self.generation_wave_pending.enqueue(coord);
        }
    }

    fn generation_wave_active(&self) -> bool {
        !self.generation_wave_targets.is_empty()
            || !self.staged_generated_chunks.is_empty()
            || !self.settled_publication_chunks.is_empty()
            || self.fluid_settling.is_active()
    }

    fn generation_wave_accepts_new_targets(&self) -> bool {
        !self.fluid_settling.is_active()
            && self.staged_generated_chunks.is_empty()
            && self.settled_publication_chunks.is_empty()
    }

    fn generation_dispatch_work_exists(&self) -> bool {
        !self.fluid_settling.is_active()
            && self.settled_publication_chunks.is_empty()
            && (self.generation_wave_pending.len() > 0
                || (self.pending.len() > 0 && self.generation_wave_accepts_new_targets()))
    }

    fn stage_generated_chunk(&mut self, coord: IVec3) {
        debug_assert!(
            self.generation_wave_targets.contains(&coord),
            "only an active generation-wave target may become staged"
        );
        self.staged_generated_chunks.insert(coord);
    }

    fn abandon_generation_wave_target(&mut self, coord: IVec3) {
        self.generation_wave_pending.remove(coord);
        self.generation_wave_targets.remove(&coord);
    }

    fn complete_generation_wave_target(&mut self, coord: IVec3) {
        debug_assert!(
            !self.staged_generated_chunks.contains(&coord),
            "completed generation-wave target cannot remain staged"
        );
        debug_assert!(
            !self.fluid_settling.contains(coord),
            "completed generation-wave target cannot remain settling-owned"
        );
        let removed = self.generation_wave_targets.remove(&coord);
        debug_assert!(
            removed,
            "completed generation-wave target must still own its reservation: {coord:?}"
        );
    }

    fn take_staged_generated_chunks(&mut self) -> Vec<IVec3> {
        let mut staged = self.staged_generated_chunks.drain().collect::<Vec<_>>();
        staged.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        staged
    }

    fn begin_settled_publication(&mut self, mut chunks: Vec<IVec3>) {
        debug_assert!(
            self.settled_publication_chunks.is_empty(),
            "settled publication queue must be empty before a new wave is staged"
        );
        chunks.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        self.settled_publication_chunks = chunks;
    }

    fn has_settled_publication(&self) -> bool {
        !self.settled_publication_chunks.is_empty()
    }

    fn pop_settled_publication_chunk(&mut self) -> Option<IVec3> {
        self.settled_publication_chunks.pop()
    }

    fn finish_generation_wave(&mut self) {
        assert!(
            self.staged_generated_chunks.is_empty(),
            "generation wave cannot finish with unpublished generated chunks"
        );
        assert!(
            self.settled_publication_chunks.is_empty(),
            "generation wave cannot finish with settled chunks awaiting publication"
        );
        assert!(
            self.generation_wave_pending.len() == 0,
            "generation wave cannot finish with unscheduled targets"
        );
        assert!(
            !self.fluid_settling.is_active(),
            "generation wave cannot finish while fluid settling is active"
        );
        assert!(
            self.generation_wave_targets.is_empty(),
            "generation wave cannot finish with unresolved target reservations: {:?}",
            self.generation_wave_targets
        );
    }

    pub(in crate::world) fn generated_chunk_is_unpublished(&self, coord: IVec3) -> bool {
        self.generation_wave_targets.contains(&coord)
            || self.staged_generated_chunks.contains(&coord)
            || self.fluid_settling.contains(coord)
    }

    pub(in crate::world) fn generated_fluid_settling_owns_mutation(&self, coord: IVec3) -> bool {
        self.fluid_settling.owns_mutation(coord)
    }

    fn resident_generated_chunk_is_unpublished(&self, coord: IVec3) -> bool {
        self.staged_generated_chunks.contains(&coord)
            || self.settled_publication_chunks.contains(&coord)
            || self.fluid_settling.contains(coord)
    }

    fn adopt_structure_top_chunk(&mut self, horizontal: IVec2, top_chunk: i32) {
        let previous_top = self
            .structure_top_chunks
            .get(&horizontal)
            .copied()
            .unwrap_or(-1);
        if top_chunk <= previous_top {
            return;
        }
        self.structure_top_chunks.insert(horizontal, top_chunk);

        let surface_top_chunk = self
            .surface_ranges
            .get(&horizontal)
            .map(|(_, maximum)| maximum.div_euclid(crate::voxel::chunk::CHUNK_SIZE as i32))
            .unwrap_or(-1);
        let start_y = previous_top.max(surface_top_chunk).saturating_add(1).max(0);
        let mut changed = false;
        for y in start_y..=top_chunk {
            let coord = IVec3::new(horizontal.x, y, horizontal.y);
            if !self.desired.insert(coord) {
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
        if self.mesh_pressure_evicted.contains_key(&coord) {
            return;
        }
        if self.keeps_loaded(coord) && !self.ready.contains(coord) {
            self.ready.enqueue(coord);
        }
    }

    pub(super) fn suppress_mesh_for_pressure(&mut self, coord: IVec3, bytes: usize) {
        self.ready.remove(coord);
        self.mesh_pressure_evicted.insert(coord, bytes);
    }

    pub(super) fn recover_mesh_after_pressure(&mut self, coord: IVec3) -> bool {
        if self.mesh_pressure_evicted.remove(&coord).is_none() || !self.keeps_loaded(coord) {
            return false;
        }
        self.mark_ready(coord);
        true
    }

    pub(super) fn mesh_pressure_evicted_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.mesh_pressure_evicted.keys().copied()
    }

    pub(super) fn mesh_pressure_evicted_bytes(&self, coord: IVec3) -> Option<usize> {
        self.mesh_pressure_evicted.get(&coord).copied()
    }

    pub(super) fn mesh_is_pressure_evicted(&self, coord: IVec3) -> bool {
        self.mesh_pressure_evicted.contains_key(&coord)
    }

    pub(super) fn retain_mesh_pressure_evictions(
        &mut self,
        desired: &HashSet<IVec3>,
        center: IVec3,
    ) {
        let before = self.mesh_pressure_evicted.len();
        self.mesh_pressure_evicted.retain(|coord, _| {
            desired.contains(coord) && !is_critical_streaming_coord(*coord, center)
        });
        if self.mesh_pressure_evicted.len() != before {
            }
    }

    fn pop_ready(&mut self) -> Option<IVec3> {
        let center = self.center?;
        let movement_direction = self.movement_direction;
        let (show_radius, _) = chunk_visibility_radii(self.horizontal_radius);
        let radius = i64::from(show_radius.max(0));
        let scan_key = SelectionScanKey {
            queue_revision: self.ready.revision(),
            selection_revision: self.selection_revision,
            center: center.xz(),
            radius_squared: radius * radius,
        };
        if self.ready_scan_miss == Some(scan_key) {
            return None;
        }

        let desired = &self.desired;
        let retained = &self.retained;
        let queue_len = self.ready.len();
        let started = Instant::now();
        let selected = self.ready.pop_min_where_by_key(
            |coord| {
                (desired.contains(&coord) || retained.contains(&coord))
                    && chunk_is_inside_render_radius(center, coord, show_radius)
            },
            |coord| chunk_load_priority(coord, center, movement_direction),
        );
        self.priority_diagnostics
            .ready
            .record(started.elapsed(), queue_len);
        if selected.is_some() {
            self.ready_scan_miss = None;
        } else {
            self.ready_scan_miss = Some(scan_key);
        }
        selected
    }

    fn defer_ready(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.ready.contains(coord) {
            self.ready.enqueue_front(coord);
        }
    }

    fn mark_initial_lighting_seeded(&mut self, coord: IVec3) -> bool {
        self.initial_lighting_seeded.insert(coord)
    }

    fn store_initial_lighting_seed_result(
        &mut self,
        coord: IVec3,
        result: DirectLightingSeedResult,
    ) {
        self.initial_lighting_seed_results.insert(coord, result);
    }

    fn take_initial_lighting_seed_result(
        &mut self,
        coord: IVec3,
    ) -> Option<DirectLightingSeedResult> {
        self.initial_lighting_seed_results.remove(&coord)
    }

    fn mark_initial_lighting_activated(&mut self, coord: IVec3) -> bool {
        self.initial_lighting_activated.insert(coord)
    }

    pub(super) fn forget_initial_lighting_seeded(&mut self, coord: IVec3) {
        self.initial_lighting_seeded.remove(&coord);
        self.initial_lighting_seed_results.remove(&coord);
        self.initial_lighting_activated.remove(&coord);
        self.initial_mesh_seed_catchup.remove(&coord);
    }


    pub(super) fn take_priority_scan_diagnostics(
        &self,
    ) -> (
        StreamingPriorityScanDiagnostic,
        StreamingPriorityScanDiagnostic,
    ) {
        (
            self.priority_diagnostics.pending.take(),
            self.priority_diagnostics.ready.take(),
        )
    }

    pub(crate) fn has_renderable_streaming_backlog(&self) -> bool {
        let Some(center) = self.center else {
            return false;
        };
        let (show_radius, _) = chunk_visibility_radii(self.horizontal_radius);
        let renderable = |coord: IVec3| {
            self.keeps_loaded(coord)
                && chunk_is_inside_render_radius(center, coord, show_radius)
        };

        self.ready.values().any(renderable)
            || self.pending.values().any(renderable)
            || self
                .generation_wave_targets
                .iter()
                .copied()
                .any(renderable)
    }

    pub(super) fn diagnostic_counts(&self) -> (usize, usize, usize, usize, usize, usize) {
        (
            self.pending.len(),
            self.ready.len(),
            self.generation_wave_pending.len(),
            self.generation_wave_targets.len(),
            self.staged_generated_chunks.len(),
            self.mesh_pressure_evicted.len(),
        )
    }

    pub(super) fn diagnostic_renderable_backlog_counts(&self) -> (usize, usize, usize) {
        let Some(center) = self.center else {
            return (0, 0, 0);
        };
        let (show_radius, _) = chunk_visibility_radii(self.horizontal_radius);
        let renderable = |coord: IVec3| {
            self.keeps_loaded(coord)
                && chunk_is_inside_render_radius(center, coord, show_radius)
        };

        (
            self.pending.values().filter(|coord| renderable(*coord)).count(),
            self.ready.values().filter(|coord| renderable(*coord)).count(),
            self.generation_wave_targets
                .iter()
                .copied()
                .filter(|coord| renderable(*coord))
                .count(),
        )
    }

    fn mark_selection_rebuilt(&mut self) {
        self.selection_revision = self
            .selection_revision
            .checked_add(1)
            .expect("chunk streaming selection revision exhausted");
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

    if work.state.center != Some(center)
        || work.state.horizontal_radius != horizontal_radius
        || work.state.vertical_radius != vertical_radius
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
                work.state.desired.len(),
                work.state.pending.len(),
                work.state.structure_top_chunks.len(),
                rebuild_elapsed.as_secs_f64() * 1_000.0,
            );
        }

        let cancelled_generation = {
            let state = &work.state;
            work.generation_tasks
                .cancel_where(|coord| !state.keeps_loaded(coord))
        };
        for coord in cancelled_generation {
            work.state.abandon_generation_wave_target(coord);
        }

        let cancelled_meshes = {
            let state = &work.state;
            work.mesh_tasks
                .cancel_where(|coord| !state.retains_render_mesh(coord))
        };
        for coord in cancelled_meshes {
            work.state.initial_mesh_seed_catchup.remove(&coord);
        }
    }

    work.generation_tasks.sync_snapshot(&generation, &content);
    work.generation_tasks.sync_streaming_region(center);
    work.mesh_tasks.sync_snapshot(&content);

    // Presentation is foreground work. Drain/publish already-built meshes and
    // feed initial meshing before integrating more generation results; otherwise
    // direct-light seeding for newly generated chunks can consume the shared
    // frame deadline while hundreds of render-ready chunks wait in `ready`.
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
    if work.generation_tasks.pending_count() > 0 || work.state.generation_wave_active() {
        collect_generated_chunks(&content, &mut work, &mut queues, current_tick);
    }
    if work.state.generation_dispatch_work_exists() {
        dispatch_generation_tasks(
            &content,
            &renderer.pool,
            &mut work,
            &mut queues,
        );
    }
}

// Direct lighting is residency safety: any loaded chunk may be sampled by
// lighting propagation, so its stored light cannot remain the all-DARK default.
// Runtime wake/relaxation queues are presentation work and are activated only
// when the chunk is actually selected for initial rendering.
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

    // A mesh task may already have captured this position as missing air.
    // Remember only the affected meshlets; the finished task can publish and
    // receive a targeted catch-up instead of being cancelled.
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
                    let combined = work
                        .state
                        .initial_mesh_seed_catchup
                        .get(&neighbor)
                        .copied()
                        .unwrap_or_default()
                        .union(meshlets);
                    work.state
                        .initial_mesh_seed_catchup
                        .insert(neighbor, combined);
                }
            }
        }
    }
}

pub(super) fn activate_loaded_chunk_for_initial_mesh(
    coord: IVec3,
    content: &ChunkContent<'_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
    seed_loaded_chunk_direct_lighting(coord, content, work, queues);
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
    #[test]
    fn ready_queue_prioritizes_distance_before_movement_direction() {
        let background = IVec3::new(-4, 0, 0);
        let forward = IVec3::new(5, 0, 0);
        let critical = IVec3::new(1, 0, 0);
        let mut state = ChunkStreamingState {
            center: Some(IVec3::ZERO),
            movement_direction: IVec2::X,
            horizontal_radius: 12,
            ..default()
        };
        state.desired.extend([background, forward, critical]);
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
        state
            .initial_mesh_seed_catchup
            .insert(coord, ChunkMeshletMask::ALL);
        state.forget_initial_lighting_seeded(coord);
        assert!(state.mark_initial_lighting_seeded(coord));
        assert!(!state.initial_mesh_seed_catchup.contains_key(&coord));
    }

    #[test]
    fn forward_preload_stays_resident_without_allocating_a_gpu_mesh() {
        let visible = IVec3::new(14, 0, 0);
        let hysteresis = IVec3::new(15, 0, 0);
        let preload_only = IVec3::new(20, 0, 0);
        let mut state = ChunkStreamingState {
            center: Some(IVec3::ZERO),
            horizontal_radius: 12,
            ..default()
        };
        state.desired.extend([visible, hysteresis, preload_only]);

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
        let mut state = ChunkStreamingState {
            center: Some(IVec3::ZERO),
            horizontal_radius: 12,
            ..default()
        };
        state.desired.insert(preload_only);
        state.mark_ready(preload_only);

        assert_eq!(state.pop_ready(), None);
        let first_miss = state.ready_scan_miss;
        assert!(first_miss.is_some());
        assert_eq!(state.pop_ready(), None);
        assert_eq!(state.ready_scan_miss, first_miss);

        let visible = IVec3::X;
        state.desired.insert(visible);
        state.mark_ready(visible);
        assert_eq!(state.pop_ready(), Some(visible));
        assert_eq!(state.ready_scan_miss, None);

        assert_eq!(state.pop_ready(), None);
        state.center = Some(IVec3::new(6, 0, 0));
        state.mark_selection_rebuilt();
        assert_eq!(state.pop_ready(), Some(preload_only));
        assert_eq!(state.ready_scan_miss, None);
    }

    #[test]
    fn critical_pending_scan_miss_invalidates_when_queue_or_center_changes() {
        let far = IVec3::new(10, 0, 0);
        let mut state = ChunkStreamingState {
            center: Some(IVec3::ZERO),
            ..default()
        };
        state.pending.enqueue(far);

        assert!(!state.has_critical_pending());
        let first_miss = state.pending_critical_scan_miss;
        assert!(first_miss.is_some());
        assert!(!state.has_critical_pending());
        assert_eq!(state.pending_critical_scan_miss, first_miss);

        state.pending.enqueue(IVec3::X);
        assert!(state.has_critical_pending());
        assert_eq!(state.pending_critical_scan_miss, None);

        state.pending.remove(IVec3::X);
        assert!(!state.has_critical_pending());
        state.center = Some(IVec3::new(9, 0, 0));
        assert!(state.has_critical_pending());
        assert_eq!(state.pending_critical_scan_miss, None);
    }

    #[test]
    fn renderable_streaming_backlog_ignores_preload_only_work() {
        let visible = IVec3::new(3, 0, 0);
        let preload_only = IVec3::new(20, 0, 0);
        let mut state = ChunkStreamingState {
            center: Some(IVec3::ZERO),
            horizontal_radius: 12,
            ..default()
        };
        state.desired.extend([visible, preload_only]);

        state.ready.enqueue(preload_only);
        assert!(!state.has_renderable_streaming_backlog());

        state.pending.enqueue(visible);
        assert!(state.has_renderable_streaming_backlog());

        state.pending.remove(visible);
        state.start_generation_wave_target(visible);
        assert!(state.has_renderable_streaming_backlog());
    }
}
