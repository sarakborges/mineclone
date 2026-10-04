use std::{cmp::Reverse, collections::VecDeque, time::Instant};

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{crash_log::append_runtime_diagnostic, game_state::GameState},
    rendering::terrain_material::TerrainMaterial,
};

use super::{
    chunk_async_work::ChunkAsyncWorkLimiter,
    chunk_remesh::ChunkRemeshQueue,
    chunk_remesh_tasks::ChunkRemeshTasks,
    chunk_rendering::ChunkRenderPool,
    streaming::ChunkStreamingState,
    warp::PendingWarp,
};

const RENDER_DIAGNOSTIC_INTERVAL_SECONDS: f32 = 10.0;
const MESH_ASSET_OVERHEAD_WARNING: usize = 128;
const FRAME_TIME_SAMPLE_CAPACITY: usize = 4096;
const SLOW_FRAME_CONTEXT_THRESHOLD_MICROS: u64 = 20_000;
const SLOW_FRAME_CONTEXT_CAPACITY: usize = 8;

#[derive(Resource, Default)]
pub(super) struct FrameTimeSamples {
    micros: VecDeque<u64>,
    slow_frames: Vec<SlowFrameContext>,
}

#[derive(Resource, Default)]
pub(super) struct MainFrameWorkSamples {
    started_at: Option<Instant>,
    micros: VecDeque<u64>,
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
    warp_active: bool,
    retired_chunks: usize,
    remesh_tasks: usize,
    async_chunk_work: usize,
    async_chunk_work_limit: usize,
    remesh_geometry: usize,
    remesh_lighting: usize,
    remesh_fluid: usize,
}

impl std::fmt::Debug for SlowFrameContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SlowFrame")
            .field("frame_us", &self.frame_micros)
            .field("warp", &self.warp_active)
            .field("retired", &self.retired_chunks)
            .field("remesh_tasks", &self.remesh_tasks)
            .field("async_work", &self.async_chunk_work)
            .field("async_limit", &self.async_chunk_work_limit)
            .field("remesh_geometry", &self.remesh_geometry)
            .field("remesh_lighting", &self.remesh_lighting)
            .field("remesh_fluid", &self.remesh_fluid)
            .finish()
    }
}

impl MainFrameWorkSamples {
    fn take_diagnostic(&mut self) -> FrameTimeDiagnostic {
        take_timing_diagnostic(&mut self.micros)
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
        take_timing_diagnostic(&mut self.micros)
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

fn take_timing_diagnostic(values: &mut VecDeque<u64>) -> FrameTimeDiagnostic {
    if values.is_empty() {
        return FrameTimeDiagnostic::default();
    }

    let mut values = values.drain(..).collect::<Vec<_>>();
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

fn percentile_micros(sorted: &[u64], percentile: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank =
        (sorted.len().saturating_mul(percentile).saturating_add(99) / 100).clamp(1, sorted.len());
    sorted[rank - 1]
}

pub(super) fn record_frame_time(time: Res<Time<Real>>, mut samples: ResMut<FrameTimeSamples>) {
    let elapsed_micros = time.delta().as_micros().min(u128::from(u64::MAX)) as u64;
    samples.record(elapsed_micros);
}

pub(super) fn begin_main_frame_work(mut samples: ResMut<MainFrameWorkSamples>) {
    samples.started_at = Some(Instant::now());
}

pub(super) fn record_main_frame_work(mut samples: ResMut<MainFrameWorkSamples>) {
    let Some(started_at) = samples.started_at.take() else {
        return;
    };
    let elapsed_micros = started_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    if samples.micros.len() >= FRAME_TIME_SAMPLE_CAPACITY {
        samples.micros.pop_front();
    }
    samples.micros.push_back(elapsed_micros);
}

pub(super) fn slow_frame_context_due(time: Res<Time<Real>>) -> bool {
    time.delta().as_micros() >= u128::from(SLOW_FRAME_CONTEXT_THRESHOLD_MICROS)
}

#[derive(SystemParam)]
pub(super) struct SlowFrameContextAssets<'w> {
    streaming: Res<'w, ChunkStreamingState>,
    async_work: Res<'w, ChunkAsyncWorkLimiter>,
    remesh_queue: Res<'w, ChunkRemeshQueue>,
    remesh_tasks: Res<'w, ChunkRemeshTasks>,
    pending_warp: Res<'w, PendingWarp>,
    samples: ResMut<'w, FrameTimeSamples>,
}

pub(super) fn record_slow_frame_context(time: Res<Time<Real>>, mut assets: SlowFrameContextAssets) {
    let frame_micros = time.delta().as_micros().min(u128::from(u64::MAX)) as u64;
    let (remesh_geometry, remesh_lighting, remesh_fluid) = assets.remesh_queue.diagnostic_counts();

    assets.samples.record_slow_frame(SlowFrameContext {
        frame_micros,
        warp_active: assets.pending_warp.streaming_center().is_some(),
        retired_chunks: assets.streaming.diagnostic_retired_count(),
        remesh_tasks: assets.remesh_tasks.pending_count(),
        async_chunk_work: assets.async_work.in_flight(),
        async_chunk_work_limit: assets.async_work.limit(),
        remesh_geometry,
        remesh_lighting,
        remesh_fluid,
    });
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RenderDiagnosticSnapshot {
    mesh_assets: usize,
    image_assets: usize,
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

#[derive(SystemParam)]
pub(super) struct RenderDiagnosticAssets<'w> {
    state: Res<'w, State<GameState>>,
    pool: Res<'w, ChunkRenderPool>,
    streaming: Res<'w, ChunkStreamingState>,
    async_work: Res<'w, ChunkAsyncWorkLimiter>,
    remesh_queue: Res<'w, ChunkRemeshQueue>,
    remesh_tasks: Res<'w, ChunkRemeshTasks>,
    meshes: Res<'w, Assets<Mesh>>,
    images: Res<'w, Assets<Image>>,
    standard_materials: Res<'w, Assets<StandardMaterial>>,
    terrain_materials: Res<'w, Assets<TerrainMaterial>>,
    frame_times: ResMut<'w, FrameTimeSamples>,
    main_frame_work: ResMut<'w, MainFrameWorkSamples>,
}

pub(super) fn log_render_asset_pressure(
    mut assets: RenderDiagnosticAssets,
    mut previous: Local<Option<RenderDiagnosticSnapshot>>,
) {
    let active_chunks = assets.pool.active_count();
    let pooled_meshes = assets.pool.mesh_count();
    let render_entities = assets.pool.entity_count();
    let pooled_mesh_bytes = assets.pool.mesh_bytes();
    let recomputed_mesh_bytes = assets.pool.diagnostic_recomputed_mesh_bytes();
    let pressure_evicted_meshes = assets.streaming.mesh_pressure_evicted_coords().count();
    let stream_retired = assets.streaming.diagnostic_retired_count();
    let frame_times = assets.frame_times.take_diagnostic();
    let main_frame_work = assets.main_frame_work.take_diagnostic();
    let slow_frames = assets.frame_times.take_slow_frames();
    let average_fps = if frame_times.average_micros == 0 {
        0.0
    } else {
        1_000_000.0 / frame_times.average_micros as f64
    };
    let async_chunk_work = assets.async_work.in_flight();
    let async_chunk_work_limit = assets.async_work.limit();
    let async_chunk_work_base_limit = assets.async_work.base_limit();
    let async_remesh = assets.async_work.take_diagnostics().remesh;
    let remesh_tasks = assets.remesh_tasks.pending_count();
    let (remesh_geometry, remesh_lighting, remesh_fluid) = assets.remesh_queue.diagnostic_counts();
    let mesh_assets = assets.meshes.len();
    let image_assets = assets.images.len();
    let mesh_overhead = mesh_assets.saturating_sub(pooled_meshes);
    let snapshot = RenderDiagnosticSnapshot {
        mesh_assets,
        image_assets,
    };
    let deltas = previous.as_ref().map(|previous| {
        (
            signed_delta(snapshot.mesh_assets, previous.mesh_assets),
            signed_delta(snapshot.image_assets, previous.image_assets),
        )
    });

    let diagnostic = format!(
        "render assets: state={:?} active_chunks={active_chunks} pooled_meshes={pooled_meshes} render_entities={render_entities} pooled_mesh_bytes={pooled_mesh_bytes} stream_retired={stream_retired} pressure_evicted_meshes={pressure_evicted_meshes} frame_samples={} frame_avg_us={} frame_avg_fps={:.1} frame_p50_us={} frame_p95_us={} frame_p99_us={} frame_max_us={} main_work_avg_us={} main_work_p50_us={} main_work_p95_us={} main_work_p99_us={} main_work_max_us={} slow_frames={slow_frames:?} async_chunk_work={async_chunk_work}/{async_chunk_work_limit} async_chunk_base_limit={async_chunk_work_base_limit} async_remesh={async_remesh:?} remesh_tasks={remesh_tasks} remesh_geometry={remesh_geometry} remesh_lighting={remesh_lighting} remesh_fluid={remesh_fluid} mesh_assets={mesh_assets} images={image_assets} standard_materials={} terrain_materials={} deltas={deltas:?}",
        assets.state.get(),
        frame_times.count,
        frame_times.average_micros,
        average_fps,
        frame_times.p50_micros,
        frame_times.p95_micros,
        frame_times.p99_micros,
        frame_times.max_micros,
        main_frame_work.average_micros,
        main_frame_work.p50_micros,
        main_frame_work.p95_micros,
        main_frame_work.p99_micros,
        main_frame_work.max_micros,
        assets.standard_materials.len(),
        assets.terrain_materials.len(),
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
}

fn signed_delta(current: usize, previous: usize) -> i64 {
    if current >= previous {
        current.saturating_sub(previous).min(i64::MAX as usize) as i64
    } else {
        -(previous.saturating_sub(current).min(i64::MAX as usize) as i64)
    }
}
