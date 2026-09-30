use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use bevy::{
    prelude::*,
    render::{Render, RenderApp, RenderSystems},
};

use crate::app::crash_log::append_runtime_diagnostic;

#[derive(Resource, Default)]
struct RenderPrepareTimer {
    started_at: Option<Instant>,
    checkpoint_at: Option<Instant>,
}

#[derive(Clone, Copy)]
enum RenderPrepareStage {
    Resources,
    BatchPhases,
    WritePhaseBuffers,
    CollectPhaseBuffers,
    Flush,
    BindGroups,
}

struct TimingMetrics {
    count: AtomicU64,
    total_nanos: AtomicU64,
    max_nanos: AtomicU64,
}

impl TimingMetrics {
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

    fn take(&self) -> TimingDiagnostic {
        let count = self.count.swap(0, Ordering::Relaxed);
        let total_nanos = self.total_nanos.swap(0, Ordering::Relaxed);
        let max_nanos = self.max_nanos.swap(0, Ordering::Relaxed);
        TimingDiagnostic {
            count,
            average_micros: total_nanos.checked_div(count).unwrap_or(0) / 1_000,
            max_micros: max_nanos / 1_000,
        }
    }
}

static PREPARE_TOTAL_METRICS: TimingMetrics = TimingMetrics::new();
static PREPARE_RESOURCES_METRICS: TimingMetrics = TimingMetrics::new();
static PREPARE_BATCH_PHASES_METRICS: TimingMetrics = TimingMetrics::new();
static PREPARE_WRITE_PHASE_BUFFERS_METRICS: TimingMetrics = TimingMetrics::new();
static PREPARE_COLLECT_PHASE_BUFFERS_METRICS: TimingMetrics = TimingMetrics::new();
static PREPARE_FLUSH_METRICS: TimingMetrics = TimingMetrics::new();
static PREPARE_BIND_GROUPS_METRICS: TimingMetrics = TimingMetrics::new();
static PREPARE_TAIL_METRICS: TimingMetrics = TimingMetrics::new();

#[derive(Clone, Copy, Default)]
struct TimingDiagnostic {
    count: u64,
    average_micros: u64,
    max_micros: u64,
}

#[derive(Clone, Copy, Default)]
struct PrepareDiagnostics {
    total: TimingDiagnostic,
    resources: TimingDiagnostic,
    batch_phases: TimingDiagnostic,
    write_phase_buffers: TimingDiagnostic,
    collect_phase_buffers: TimingDiagnostic,
    flush: TimingDiagnostic,
    bind_groups: TimingDiagnostic,
    tail: TimingDiagnostic,
}

pub(super) fn install_render_prepare_diagnostics(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };

    render_app
        .init_resource::<RenderPrepareTimer>()
        .add_systems(
            Render,
            begin_prepare_breakdown
                .in_set(RenderSystems::Prepare)
                .before(RenderSystems::PrepareResources),
        )
        .add_systems(
            Render,
            checkpoint_prepare_resources
                .in_set(RenderSystems::Prepare)
                .after(RenderSystems::PrepareResources)
                .before(RenderSystems::PrepareResourcesBatchPhases),
        )
        .add_systems(
            Render,
            checkpoint_prepare_batch_phases
                .in_set(RenderSystems::Prepare)
                .after(RenderSystems::PrepareResourcesBatchPhases)
                .before(RenderSystems::PrepareResourcesWritePhaseBuffers),
        )
        .add_systems(
            Render,
            checkpoint_prepare_write_phase_buffers
                .in_set(RenderSystems::Prepare)
                .after(RenderSystems::PrepareResourcesWritePhaseBuffers)
                .before(RenderSystems::PrepareResourcesCollectPhaseBuffers),
        )
        .add_systems(
            Render,
            checkpoint_prepare_collect_phase_buffers
                .in_set(RenderSystems::Prepare)
                .after(RenderSystems::PrepareResourcesCollectPhaseBuffers)
                .before(RenderSystems::PrepareResourcesFlush),
        )
        .add_systems(
            Render,
            checkpoint_prepare_flush
                .in_set(RenderSystems::Prepare)
                .after(RenderSystems::PrepareResourcesFlush)
                .before(RenderSystems::PrepareBindGroups),
        )
        .add_systems(
            Render,
            checkpoint_prepare_bind_groups
                .in_set(RenderSystems::Prepare)
                .after(RenderSystems::PrepareBindGroups),
        )
        .add_systems(
            Render,
            finish_prepare_breakdown
                .after(RenderSystems::Prepare)
                .before(RenderSystems::Render),
        );
}

fn begin_prepare_breakdown(mut timer: ResMut<RenderPrepareTimer>) {
    let now = Instant::now();
    timer.started_at = Some(now);
    timer.checkpoint_at = Some(now);
}

fn checkpoint_prepare_resources(mut timer: ResMut<RenderPrepareTimer>) {
    checkpoint_prepare_stage(&mut timer, RenderPrepareStage::Resources);
}

fn checkpoint_prepare_batch_phases(mut timer: ResMut<RenderPrepareTimer>) {
    checkpoint_prepare_stage(&mut timer, RenderPrepareStage::BatchPhases);
}

fn checkpoint_prepare_write_phase_buffers(mut timer: ResMut<RenderPrepareTimer>) {
    checkpoint_prepare_stage(&mut timer, RenderPrepareStage::WritePhaseBuffers);
}

fn checkpoint_prepare_collect_phase_buffers(mut timer: ResMut<RenderPrepareTimer>) {
    checkpoint_prepare_stage(&mut timer, RenderPrepareStage::CollectPhaseBuffers);
}

fn checkpoint_prepare_flush(mut timer: ResMut<RenderPrepareTimer>) {
    checkpoint_prepare_stage(&mut timer, RenderPrepareStage::Flush);
}

fn checkpoint_prepare_bind_groups(mut timer: ResMut<RenderPrepareTimer>) {
    checkpoint_prepare_stage(&mut timer, RenderPrepareStage::BindGroups);
}

fn checkpoint_prepare_stage(timer: &mut RenderPrepareTimer, stage: RenderPrepareStage) {
    let finished_at = Instant::now();
    let Some(started_at) = timer.checkpoint_at.replace(finished_at) else {
        return;
    };
    prepare_stage_metrics(stage).record(elapsed_nanos(started_at, finished_at));
}

fn finish_prepare_breakdown(mut timer: ResMut<RenderPrepareTimer>) {
    let finished_at = Instant::now();
    if let Some(checkpoint_at) = timer.checkpoint_at.take() {
        PREPARE_TAIL_METRICS.record(elapsed_nanos(checkpoint_at, finished_at));
    }
    if let Some(started_at) = timer.started_at.take() {
        PREPARE_TOTAL_METRICS.record(elapsed_nanos(started_at, finished_at));
    }
}

fn prepare_stage_metrics(stage: RenderPrepareStage) -> &'static TimingMetrics {
    match stage {
        RenderPrepareStage::Resources => &PREPARE_RESOURCES_METRICS,
        RenderPrepareStage::BatchPhases => &PREPARE_BATCH_PHASES_METRICS,
        RenderPrepareStage::WritePhaseBuffers => &PREPARE_WRITE_PHASE_BUFFERS_METRICS,
        RenderPrepareStage::CollectPhaseBuffers => &PREPARE_COLLECT_PHASE_BUFFERS_METRICS,
        RenderPrepareStage::Flush => &PREPARE_FLUSH_METRICS,
        RenderPrepareStage::BindGroups => &PREPARE_BIND_GROUPS_METRICS,
    }
}

fn elapsed_nanos(started_at: Instant, finished_at: Instant) -> u64 {
    finished_at
        .duration_since(started_at)
        .as_nanos()
        .min(u128::from(u64::MAX)) as u64
}

fn take_prepare_diagnostics() -> PrepareDiagnostics {
    PrepareDiagnostics {
        total: PREPARE_TOTAL_METRICS.take(),
        resources: PREPARE_RESOURCES_METRICS.take(),
        batch_phases: PREPARE_BATCH_PHASES_METRICS.take(),
        write_phase_buffers: PREPARE_WRITE_PHASE_BUFFERS_METRICS.take(),
        collect_phase_buffers: PREPARE_COLLECT_PHASE_BUFFERS_METRICS.take(),
        flush: PREPARE_FLUSH_METRICS.take(),
        bind_groups: PREPARE_BIND_GROUPS_METRICS.take(),
        tail: PREPARE_TAIL_METRICS.take(),
    }
}

pub(super) fn reset_render_prepare_diagnostics() {
    let _ = take_prepare_diagnostics();
}

pub(super) fn log_render_prepare_work() {
    let diagnostic = take_prepare_diagnostics();
    let line = format!(
        "render prepare: samples={} total_avg_us={} total_max_us={} resources_avg_us={} resources_max_us={} batch_phases_avg_us={} batch_phases_max_us={} write_phase_buffers_avg_us={} write_phase_buffers_max_us={} collect_phase_buffers_avg_us={} collect_phase_buffers_max_us={} flush_avg_us={} flush_max_us={} bind_groups_avg_us={} bind_groups_max_us={} tail_avg_us={} tail_max_us={}",
        diagnostic.total.count,
        diagnostic.total.average_micros,
        diagnostic.total.max_micros,
        diagnostic.resources.average_micros,
        diagnostic.resources.max_micros,
        diagnostic.batch_phases.average_micros,
        diagnostic.batch_phases.max_micros,
        diagnostic.write_phase_buffers.average_micros,
        diagnostic.write_phase_buffers.max_micros,
        diagnostic.collect_phase_buffers.average_micros,
        diagnostic.collect_phase_buffers.max_micros,
        diagnostic.flush.average_micros,
        diagnostic.flush.max_micros,
        diagnostic.bind_groups.average_micros,
        diagnostic.bind_groups.max_micros,
        diagnostic.tail.average_micros,
        diagnostic.tail.max_micros,
    );
    info!("{line}");
    let _ = append_runtime_diagnostic(&line);
}
