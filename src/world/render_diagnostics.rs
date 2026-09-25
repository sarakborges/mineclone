use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};

use bevy::{ecs::system::SystemParam, prelude::*, text::FontAtlasSet};

use crate::{
    app::{crash_log::append_runtime_diagnostic, game_state::GameState},
    rendering::{
        extruded_sprite::{ExtrudedSpriteMaterialCache, ExtrudedSpriteMeshCache},
        terrain_material::TerrainMaterial,
    },
    world_objects::{ObjectMaterialCache, WorldObjectStore},
};

use super::{
    chunk_async_work::ChunkAsyncWorkLimiter,
    chunk_generation_tasks::ChunkGenerationTasks,
    chunk_mesh_tasks::ChunkMeshTasks,
    chunk_remesh::ChunkRemeshQueue,
    chunk_remesh_tasks::ChunkRemeshTasks,
    chunk_rendering::ChunkRenderPool,
    streaming::ChunkStreamingState,
    warp::PendingWarp,
};

const RENDER_DIAGNOSTIC_INTERVAL_SECONDS: f32 = 10.0;
const MESH_ASSET_OVERHEAD_WARNING: usize = 128;
const RUNTIME_IMAGE_SHAPE_LIMIT: usize = 4;
const FONT_ATLAS_SIZE_SAMPLE_LIMIT: usize = 8;
const FRAME_TIME_SAMPLE_CAPACITY: usize = 4096;
const SLOW_FRAME_CONTEXT_THRESHOLD_MICROS: u64 = 20_000;
const SLOW_FRAME_CONTEXT_CAPACITY: usize = 8;

#[derive(Resource, Default)]
pub(super) struct FrameTimeSamples {
    micros: VecDeque<u64>,
    slow_frames: Vec<SlowFrameContext>,
}

#[derive(Clone, Copy, Debug, Default)]
struct FrameTimeDiagnostic {
    count: usize,
    average_micros: u64,
    p50_micros: u64,
    p95_micros: u64,
    p99_micros: u64,
    max_micros: u64,
}

#[derive(Clone, Copy, Default)]
struct SlowFrameContext {
    frame_micros: u64,
    selection_revision: u64,
    warp_active: bool,
    stream_pending: usize,
    stream_ready: usize,
    generation_tasks: usize,
    mesh_tasks: usize,
    remesh_tasks: usize,
    async_chunk_work: usize,
    async_chunk_work_limit: usize,
    generation_wave_pending: usize,
    generation_wave_targets: usize,
    staged_generated_chunks: usize,
    remesh_geometry: usize,
    remesh_lighting: usize,
    remesh_fluid: usize,
}

impl std::fmt::Debug for SlowFrameContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SlowFrame")
            .field("frame_us", &self.frame_micros)
            .field("selection_revision", &self.selection_revision)
            .field("warp", &self.warp_active)
            .field("pending", &self.stream_pending)
            .field("ready", &self.stream_ready)
            .field("generation_tasks", &self.generation_tasks)
            .field("mesh_tasks", &self.mesh_tasks)
            .field("remesh_tasks", &self.remesh_tasks)
            .field("async_work", &self.async_chunk_work)
            .field("async_limit", &self.async_chunk_work_limit)
            .field("wave_pending", &self.generation_wave_pending)
            .field("wave_targets", &self.generation_wave_targets)
            .field("staged", &self.staged_generated_chunks)
            .field("remesh_geometry", &self.remesh_geometry)
            .field("remesh_lighting", &self.remesh_lighting)
            .field("remesh_fluid", &self.remesh_fluid)
            .finish()
    }
}

impl FrameTimeSamples {
    fn record(&mut self, elapsed_micros: u64) {
        if self.micros.len() >= FRAME_TIME_SAMPLE_CAPACITY {
            self.micros.pop_front();
        }
        self.micros.push_back(elapsed_micros);
    }

    fn take_diagnostic(&mut self) -> FrameTimeDiagnostic {
        if self.micros.is_empty() {
            return FrameTimeDiagnostic::default();
        }

        let mut values = self.micros.drain(..).collect::<Vec<_>>();
        values.sort_unstable();
        let count = values.len();
        let total = values
            .iter()
            .fold(0_u128, |sum, value| sum + u128::from(*value));
        FrameTimeDiagnostic {
            count,
            average_micros: (total / count as u128).min(u128::from(u64::MAX)) as u64,
            p50_micros: percentile_micros(&values, 50),
            p95_micros: percentile_micros(&values, 95),
            p99_micros: percentile_micros(&values, 99),
            max_micros: values.last().copied().unwrap_or(0),
        }
    }

    fn record_slow_frame(&mut self, context: SlowFrameContext) {
        self.slow_frames.push(context);
        self.slow_frames
            .sort_unstable_by_key(|context| Reverse(context.frame_micros));
        self.slow_frames.truncate(SLOW_FRAME_CONTEXT_CAPACITY);
    }

    fn take_slow_frames(&mut self) -> Vec<SlowFrameContext> {
        std::mem::take(&mut self.slow_frames)
    }
}

fn percentile_micros(sorted: &[u64], percentile: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (sorted.len().saturating_mul(percentile).saturating_add(99) / 100)
        .clamp(1, sorted.len());
    sorted[rank - 1]
}

pub(super) fn record_frame_time(
    time: Res<Time<Real>>,
    mut samples: ResMut<FrameTimeSamples>,
) {
    let elapsed_micros = time.delta().as_micros().min(u128::from(u64::MAX)) as u64;
    samples.record(elapsed_micros);
}

pub(super) fn slow_frame_context_due(time: Res<Time<Real>>) -> bool {
    time.delta().as_micros() >= u128::from(SLOW_FRAME_CONTEXT_THRESHOLD_MICROS)
}

#[derive(SystemParam)]
pub(super) struct SlowFrameContextAssets<'w> {
    streaming: Res<'w, ChunkStreamingState>,
    generation_tasks: Res<'w, ChunkGenerationTasks>,
    async_work: Res<'w, ChunkAsyncWorkLimiter>,
    mesh_tasks: Res<'w, ChunkMeshTasks>,
    remesh_queue: Res<'w, ChunkRemeshQueue>,
    remesh_tasks: Res<'w, ChunkRemeshTasks>,
    pending_warp: Res<'w, PendingWarp>,
    samples: ResMut<'w, FrameTimeSamples>,
}

pub(super) fn record_slow_frame_context(
    time: Res<Time<Real>>,
    mut assets: SlowFrameContextAssets,
) {
    let frame_micros = time.delta().as_micros().min(u128::from(u64::MAX)) as u64;
    let (
        stream_pending,
        stream_ready,
        generation_wave_pending,
        generation_wave_targets,
        staged_generated_chunks,
        _,
    ) = assets.streaming.diagnostic_counts();
    let (remesh_geometry, remesh_lighting, remesh_fluid) =
        assets.remesh_queue.diagnostic_counts();

    assets.samples.record_slow_frame(SlowFrameContext {
        frame_micros,
        selection_revision: assets.streaming.selection_revision(),
        warp_active: assets.pending_warp.streaming_center().is_some(),
        stream_pending,
        stream_ready,
        generation_tasks: assets.generation_tasks.pending_count(),
        mesh_tasks: assets.mesh_tasks.pending_count(),
        remesh_tasks: assets.remesh_tasks.pending_count(),
        async_chunk_work: assets.async_work.in_flight(),
        async_chunk_work_limit: assets.async_work.limit(),
        generation_wave_pending,
        generation_wave_targets,
        staged_generated_chunks,
        remesh_geometry,
        remesh_lighting,
        remesh_fluid,
    });
}

#[derive(Debug)]
struct FontAtlasKeyDiagnostic {
    faces: usize,
    sizes: usize,
    variations: usize,
    raster_modes: usize,
    size_range: Option<(f32, f32)>,
    top_sizes: Vec<(f32, usize)>,
}

fn font_atlas_key_diagnostic(font_atlases: &FontAtlasSet) -> FontAtlasKeyDiagnostic {
    let mut faces = HashSet::new();
    let mut size_counts = HashMap::<u32, usize>::new();
    let mut variations = HashSet::new();
    let mut raster_modes = HashSet::new();

    for key in font_atlases.keys() {
        faces.insert((key.id, key.index));
        *size_counts.entry(key.font_size_bits).or_default() += 1;
        variations.insert(key.variations_hash);
        raster_modes.insert((key.hinting, key.font_smoothing));
    }

    let mut sizes = size_counts
        .iter()
        .map(|(&bits, &count)| (bits, count))
        .collect::<Vec<_>>();
    sizes.sort_unstable_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let top_sizes = sizes
        .iter()
        .take(FONT_ATLAS_SIZE_SAMPLE_LIMIT)
        .map(|(bits, count)| (f32::from_bits(*bits), *count))
        .collect();

    let mut resolved_sizes = size_counts
        .keys()
        .copied()
        .map(f32::from_bits)
        .collect::<Vec<_>>();
    resolved_sizes.sort_by(f32::total_cmp);
    let size_range = resolved_sizes
        .first()
        .copied()
        .zip(resolved_sizes.last().copied());

    FontAtlasKeyDiagnostic {
        faces: faces.len(),
        sizes: size_counts.len(),
        variations: variations.len(),
        raster_modes: raster_modes.len(),
        size_range,
        top_sizes,
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RenderDiagnosticSnapshot {
    runtime_images: usize,
    font_atlases: usize,
    font_atlas_bytes: u64,
    non_font_runtime_images: usize,
}

#[derive(SystemParam)]
pub(super) struct RenderDiagnosticAssets<'w> {
    state: Res<'w, State<GameState>>,
    asset_server: Res<'w, AssetServer>,
    pool: Res<'w, ChunkRenderPool>,
    streaming: Res<'w, ChunkStreamingState>,
    generation_tasks: Res<'w, ChunkGenerationTasks>,
    async_work: Res<'w, ChunkAsyncWorkLimiter>,
    mesh_tasks: Res<'w, ChunkMeshTasks>,
    remesh_queue: Res<'w, ChunkRemeshQueue>,
    remesh_tasks: Res<'w, ChunkRemeshTasks>,
    meshes: Res<'w, Assets<Mesh>>,
    images: Res<'w, Assets<Image>>,
    font_atlases: Res<'w, FontAtlasSet>,
    standard_materials: Res<'w, Assets<StandardMaterial>>,
    terrain_materials: Res<'w, Assets<TerrainMaterial>>,
    world_objects: Res<'w, WorldObjectStore>,
    object_materials: Res<'w, ObjectMaterialCache>,
    extruded_sprite_meshes: Res<'w, ExtrudedSpriteMeshCache>,
    extruded_sprite_materials: Res<'w, ExtrudedSpriteMaterialCache>,
    frame_times: ResMut<'w, FrameTimeSamples>,
}

pub(super) fn render_diagnostics_due(
    state: Res<State<GameState>>,
    time: Res<Time<Real>>,
    mut timer: Local<Option<Timer>>,
) -> bool {
    if !matches!(state.get(), GameState::Loading | GameState::Gameplay) {
        return false;
    }

    let timer = timer.get_or_insert_with(|| {
        Timer::from_seconds(RENDER_DIAGNOSTIC_INTERVAL_SECONDS, TimerMode::Repeating)
    });
    timer.tick(time.delta()).just_finished()
}

pub(super) fn log_render_asset_pressure(
    mut assets: RenderDiagnosticAssets,
    mut previous: Local<Option<RenderDiagnosticSnapshot>>,
    mut previous_diagnostic_micros: Local<u64>,
) {
    let diagnostic_started = Instant::now();
    let active_chunks = assets.pool.active_count();
    let pooled_meshes = assets.pool.mesh_count();
    let render_entities = assets.pool.entity_count();
    let (terrain_array_meshes, terrain_legacy_meshes, layer_meshes, fluid_meshes) =
        assets.pool.diagnostic_mesh_kind_counts();
    let pooled_mesh_bytes = assets.pool.mesh_bytes();
    let recomputed_mesh_bytes = assets.pool.diagnostic_recomputed_mesh_bytes();
    let (
        stream_pending,
        stream_ready,
        generation_wave_pending,
        generation_wave_targets,
        staged_generated_chunks,
        pressure_evicted_meshes,
    ) = assets.streaming.diagnostic_counts();
    let (stream_pending_renderable, stream_ready_renderable, wave_targets_renderable) =
        assets.streaming.diagnostic_renderable_backlog_counts();
    let (pending_priority_scan, ready_priority_scan) =
        assets.streaming.take_priority_scan_diagnostics();
    let frame_times = assets.frame_times.take_diagnostic();
    let slow_frames = assets.frame_times.take_slow_frames();
    let average_fps = if frame_times.average_micros == 0 {
        0.0
    } else {
        1_000_000.0 / frame_times.average_micros as f64
    };
    let generation_tasks = assets.generation_tasks.pending_count();
    let async_chunk_work = assets.async_work.in_flight();
    let async_chunk_work_limit = assets.async_work.limit();
    let async_chunk_work_base_limit = assets.async_work.base_limit();
    let async_timings = assets.async_work.take_diagnostics();
    let mesh_tasks = assets.mesh_tasks.pending_count();
    let remesh_tasks = assets.remesh_tasks.pending_count();
    let (remesh_geometry, remesh_lighting, remesh_fluid) =
        assets.remesh_queue.diagnostic_counts();
    let mesh_assets = assets.meshes.len();
    let mesh_overhead = mesh_assets.saturating_sub(pooled_meshes);
    let mut file_images = 0;
    let mut runtime_image_shapes = HashMap::<(u32, u32), usize>::new();

    for (id, image) in assets.images.iter() {
        if assets.asset_server.get_path(id).is_some() {
            file_images += 1;
            continue;
        }

        let size = image.texture_descriptor.size;
        *runtime_image_shapes
            .entry((size.width, size.height))
            .or_default() += 1;
    }

    let image_assets = assets.images.len();
    let runtime_images = image_assets.saturating_sub(file_images);
    let mut runtime_top_shapes = runtime_image_shapes.into_iter().collect::<Vec<_>>();
    runtime_top_shapes.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    runtime_top_shapes.truncate(RUNTIME_IMAGE_SHAPE_LIMIT);

    let font_atlas_keys = assets.font_atlases.len();
    let font_atlas_key_diagnostic = font_atlas_key_diagnostic(&assets.font_atlases);
    let font_atlas_count = assets.font_atlases.values().map(Vec::len).sum::<usize>();
    let font_atlas_bytes = assets.font_atlases.total_bytes(&assets.images);
    let non_font_runtime_images = runtime_images.saturating_sub(font_atlas_count);
    let snapshot = RenderDiagnosticSnapshot {
        runtime_images,
        font_atlases: font_atlas_count,
        font_atlas_bytes,
        non_font_runtime_images,
    };
    let deltas = previous.as_ref().map(|previous| {
        (
            signed_delta(snapshot.runtime_images, previous.runtime_images),
            signed_delta(snapshot.font_atlases, previous.font_atlases),
            signed_delta_u64(snapshot.font_atlas_bytes, previous.font_atlas_bytes),
            signed_delta(
                snapshot.non_font_runtime_images,
                previous.non_font_runtime_images,
            ),
        )
    });

    let diagnostic = format!(
        "render assets: state={:?} active_chunks={active_chunks} pooled_meshes={pooled_meshes} render_entities={render_entities} terrain_array_meshes={terrain_array_meshes} terrain_legacy_meshes={terrain_legacy_meshes} layer_meshes={layer_meshes} fluid_meshes={fluid_meshes} pooled_mesh_bytes={pooled_mesh_bytes} diagnostic_prev_us={} frame_samples={} frame_avg_us={} frame_avg_fps={:.1} frame_p50_us={} frame_p95_us={} frame_p99_us={} frame_max_us={} slow_frames={slow_frames:?} stream_pending={stream_pending} stream_pending_renderable={stream_pending_renderable} stream_ready={stream_ready} stream_ready_renderable={stream_ready_renderable} wave_targets_renderable={wave_targets_renderable} pending_priority_scans={} pending_priority_avg_us={} pending_priority_max_us={} pending_priority_max_queue={} ready_priority_scans={} ready_priority_avg_us={} ready_priority_max_us={} ready_priority_max_queue={} generation_tasks={generation_tasks} async_chunk_work={async_chunk_work}/{async_chunk_work_limit} async_chunk_base_limit={async_chunk_work_base_limit} async_generation={:?} async_initial_mesh={:?} async_remesh={:?} generation_wave_pending={generation_wave_pending} generation_wave_targets={generation_wave_targets} staged_generated_chunks={staged_generated_chunks} pressure_evicted_meshes={pressure_evicted_meshes} mesh_tasks={mesh_tasks} remesh_tasks={remesh_tasks} remesh_geometry={remesh_geometry} remesh_lighting={remesh_lighting} remesh_fluid={remesh_fluid} mesh_assets={mesh_assets} images={image_assets} file_images={file_images} runtime_images={runtime_images} non_font_runtime_images={non_font_runtime_images} runtime_top_shapes={runtime_top_shapes:?} font_atlas_keys={font_atlas_keys} font_atlas_faces={} font_atlas_sizes={} font_atlas_variations={} font_atlas_raster_modes={} font_atlas_size_range={:?} font_atlas_top_sizes={:?} font_atlases={font_atlas_count} font_atlas_bytes={font_atlas_bytes} deltas={deltas:?} standard_materials={} terrain_materials={} world_objects={} world_object_chunks={} object_material_cache={} extruded_sprite_mesh_cache={} extruded_sprite_material_cache={}",
        assets.state.get(),
        *previous_diagnostic_micros,
        frame_times.count,
        frame_times.average_micros,
        average_fps,
        frame_times.p50_micros,
        frame_times.p95_micros,
        frame_times.p99_micros,
        frame_times.max_micros,
        pending_priority_scan.count,
        pending_priority_scan.average_micros,
        pending_priority_scan.max_micros,
        pending_priority_scan.max_queue_len,
        ready_priority_scan.count,
        ready_priority_scan.average_micros,
        ready_priority_scan.max_micros,
        ready_priority_scan.max_queue_len,
        async_timings.generation,
        async_timings.initial_mesh,
        async_timings.remesh,
        font_atlas_key_diagnostic.faces,
        font_atlas_key_diagnostic.sizes,
        font_atlas_key_diagnostic.variations,
        font_atlas_key_diagnostic.raster_modes,
        font_atlas_key_diagnostic.size_range,
        font_atlas_key_diagnostic.top_sizes,
        assets.standard_materials.len(),
        assets.terrain_materials.len(),
        assets.world_objects.materialized_object_count(),
        assets.world_objects.materialized_chunk_count(),
        assets.object_materials.len(),
        assets.extruded_sprite_meshes.len(),
        assets.extruded_sprite_materials.len(),
    );
    info!("{diagnostic}");
    let _ = append_runtime_diagnostic(&diagnostic);

    *previous = Some(snapshot);

    if pooled_mesh_bytes != recomputed_mesh_bytes {
        warn!(
            "render asset pressure: incremental mesh bytes drifted: tracked={pooled_mesh_bytes} recomputed={recomputed_mesh_bytes}"
        );
    }

    if !assets.pool.diagnostic_active_columns_are_consistent() {
        warn!("render asset pressure: incremental active render-column counts drifted");
    }

    if mesh_overhead > MESH_ASSET_OVERHEAD_WARNING {
        warn!(
            "render asset pressure: mesh asset overhead is {mesh_overhead} above the chunk render pool"
        );
    }

    *previous_diagnostic_micros = diagnostic_started
        .elapsed()
        .as_micros()
        .min(u128::from(u64::MAX)) as u64;
}

fn signed_delta(current: usize, previous: usize) -> i64 {
    if current >= previous {
        current.saturating_sub(previous).min(i64::MAX as usize) as i64
    } else {
        -(previous.saturating_sub(current).min(i64::MAX as usize) as i64)
    }
}

fn signed_delta_u64(current: u64, previous: u64) -> i64 {
    if current >= previous {
        current.saturating_sub(previous).min(i64::MAX as u64) as i64
    } else {
        -(previous.saturating_sub(current).min(i64::MAX as u64) as i64)
    }
}
