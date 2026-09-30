use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use bevy::{
    prelude::*,
    render::{Render, RenderApp, RenderSystems},
};

use crate::app::{crash_log::append_runtime_diagnostic, game_state::GameState};

const RENDER_WORK_SAMPLE_CAPACITY: usize = 4096;
const RENDER_WORK_MICROS_BITS: u32 = 32;
const RENDER_WORK_MICROS_MASK: u64 = u32::MAX as u64;

#[derive(Resource, Clone, Default)]
pub(super) struct RenderFrameWorkBridge(Arc<AtomicU64>);

#[derive(Resource, Default)]
struct RenderFrameWorkTimer {
    started_at: Option<Instant>,
    sequence: u32,
}

#[derive(Resource, Default)]
pub(super) struct RenderFrameWorkSamples {
    last_sequence: u32,
    skipped_samples: u64,
    micros: VecDeque<u64>,
}

#[derive(Clone, Copy, Debug, Default)]
struct RenderFrameWorkDiagnostic {
    count: usize,
    skipped_samples: u64,
    average_micros: u64,
    p50_micros: u64,
    p95_micros: u64,
    p99_micros: u64,
    max_micros: u64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum PresentationPublicationStage {
    InitialPublish,
    RemeshApply,
}

struct PresentationPublicationStageMetrics {
    count: AtomicU64,
    total_nanos: AtomicU64,
    max_nanos: AtomicU64,
}

impl PresentationPublicationStageMetrics {
    const fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            total_nanos: AtomicU64::new(0),
            max_nanos: AtomicU64::new(0),
        }
    }

    fn record(&self, elapsed_nanos: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.total_nanos
            .fetch_add(elapsed_nanos, Ordering::Relaxed);
        self.max_nanos.fetch_max(elapsed_nanos, Ordering::Relaxed);
    }

    fn take(&self) -> PresentationPublicationStageDiagnostic {
        let count = self.count.swap(0, Ordering::Relaxed);
        let total_nanos = self.total_nanos.swap(0, Ordering::Relaxed);
        let max_nanos = self.max_nanos.swap(0, Ordering::Relaxed);
        PresentationPublicationStageDiagnostic {
            count,
            average_micros: total_nanos.checked_div(count).unwrap_or(0) / 1_000,
            max_micros: max_nanos / 1_000,
        }
    }
}

static PRESENTATION_INITIAL_PUBLISH_METRICS: PresentationPublicationStageMetrics =
    PresentationPublicationStageMetrics::new();
static PRESENTATION_REMESH_APPLY_METRICS: PresentationPublicationStageMetrics =
    PresentationPublicationStageMetrics::new();

#[derive(Clone, Copy, Default)]
struct PresentationPublicationStageDiagnostic {
    count: u64,
    average_micros: u64,
    max_micros: u64,
}

pub(crate) struct PresentationPublicationTimer {
    stage: PresentationPublicationStage,
    started_at: Instant,
}

impl PresentationPublicationTimer {
    pub(crate) fn start(stage: PresentationPublicationStage) -> Self {
        Self {
            stage,
            started_at: Instant::now(),
        }
    }
}

impl Drop for PresentationPublicationTimer {
    fn drop(&mut self) {
        let elapsed_nanos = self
            .started_at
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;
        publication_metrics(self.stage).record(elapsed_nanos);
    }
}

fn publication_metrics(
    stage: PresentationPublicationStage,
) -> &'static PresentationPublicationStageMetrics {
    match stage {
        PresentationPublicationStage::InitialPublish => &PRESENTATION_INITIAL_PUBLISH_METRICS,
        PresentationPublicationStage::RemeshApply => &PRESENTATION_REMESH_APPLY_METRICS,
    }
}

fn take_presentation_publication_diagnostics() -> (
    PresentationPublicationStageDiagnostic,
    PresentationPublicationStageDiagnostic,
) {
    (
        PRESENTATION_INITIAL_PUBLISH_METRICS.take(),
        PRESENTATION_REMESH_APPLY_METRICS.take(),
    )
}

pub(super) fn install_render_work_diagnostics(app: &mut App) {
    let bridge = RenderFrameWorkBridge::default();
    app.insert_resource(bridge.clone())
        .init_resource::<RenderFrameWorkSamples>();

    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };

    render_app
        .insert_resource(bridge)
        .init_resource::<RenderFrameWorkTimer>()
        .add_systems(
            Render,
            begin_render_frame_work.before(RenderSystems::ExtractCommands),
        )
        .add_systems(
            Render,
            record_render_frame_work.after(RenderSystems::PostCleanup),
        );
}

fn begin_render_frame_work(mut timer: ResMut<RenderFrameWorkTimer>) {
    timer.started_at = Some(Instant::now());
}

fn record_render_frame_work(
    mut timer: ResMut<RenderFrameWorkTimer>,
    bridge: Res<RenderFrameWorkBridge>,
) {
    let Some(started_at) = timer.started_at.take() else {
        return;
    };

    timer.sequence = timer.sequence.wrapping_add(1);
    if timer.sequence == 0 {
        timer.sequence = 1;
    }

    let elapsed_micros = started_at
        .elapsed()
        .as_micros()
        .min(u128::from(u32::MAX)) as u64;
    bridge.0.store(
        (u64::from(timer.sequence) << RENDER_WORK_MICROS_BITS) | elapsed_micros,
        Ordering::Release,
    );
}

pub(super) fn collect_render_frame_work(
    bridge: Res<RenderFrameWorkBridge>,
    mut samples: ResMut<RenderFrameWorkSamples>,
) {
    let packed_sample = bridge.0.load(Ordering::Acquire);
    let sequence = (packed_sample >> RENDER_WORK_MICROS_BITS) as u32;
    if sequence == 0 || sequence == samples.last_sequence {
        return;
    }

    if samples.last_sequence != 0 {
        let advanced = sequence.wrapping_sub(samples.last_sequence);
        if advanced > 1 {
            samples.skipped_samples = samples
                .skipped_samples
                .saturating_add(u64::from(advanced - 1));
        }
    }
    samples.last_sequence = sequence;

    if samples.micros.len() >= RENDER_WORK_SAMPLE_CAPACITY {
        samples.micros.pop_front();
    }
    samples
        .micros
        .push_back(packed_sample & RENDER_WORK_MICROS_MASK);
}

pub(super) fn reset_render_frame_work_samples(
    state: Res<State<GameState>>,
    bridge: Res<RenderFrameWorkBridge>,
    mut samples: ResMut<RenderFrameWorkSamples>,
) {
    let packed_sample = bridge.0.load(Ordering::Acquire);
    samples.last_sequence = (packed_sample >> RENDER_WORK_MICROS_BITS) as u32;
    samples.skipped_samples = 0;
    samples.micros.clear();
    if *state.get() == GameState::Loading {
        let _ = take_presentation_publication_diagnostics();
    }
}

pub(super) fn log_render_frame_work(mut samples: ResMut<RenderFrameWorkSamples>) {
    let diagnostic = samples.take_diagnostic();
    let (publish_initial, publish_remesh_apply) = take_presentation_publication_diagnostics();
    let line = format!(
        "render work: samples={} skipped_samples={} avg_us={} p50_us={} p95_us={} p99_us={} max_us={} publish_initial_count={} publish_initial_avg_us={} publish_initial_max_us={} publish_remesh_apply_count={} publish_remesh_apply_avg_us={} publish_remesh_apply_max_us={}",
        diagnostic.count,
        diagnostic.skipped_samples,
        diagnostic.average_micros,
        diagnostic.p50_micros,
        diagnostic.p95_micros,
        diagnostic.p99_micros,
        diagnostic.max_micros,
        publish_initial.count,
        publish_initial.average_micros,
        publish_initial.max_micros,
        publish_remesh_apply.count,
        publish_remesh_apply.average_micros,
        publish_remesh_apply.max_micros,
    );
    info!("{line}");
    let _ = append_runtime_diagnostic(&line);
}

impl RenderFrameWorkSamples {
    fn take_diagnostic(&mut self) -> RenderFrameWorkDiagnostic {
        let skipped_samples = std::mem::take(&mut self.skipped_samples);
        if self.micros.is_empty() {
            return RenderFrameWorkDiagnostic {
                skipped_samples,
                ..default()
            };
        }

        let mut values = self.micros.drain(..).collect::<Vec<_>>();
        values.sort_unstable();
        let count = values.len();
        let total = values
            .iter()
            .fold(0_u128, |sum, value| sum + u128::from(*value));

        RenderFrameWorkDiagnostic {
            count,
            skipped_samples,
            average_micros: (total / count as u128).min(u128::from(u64::MAX)) as u64,
            p50_micros: percentile_micros(&values, 50),
            p95_micros: percentile_micros(&values, 95),
            p99_micros: percentile_micros(&values, 99),
            max_micros: values.last().copied().unwrap_or(0),
        }
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
