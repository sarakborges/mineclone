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

use crate::app::crash_log::append_runtime_diagnostic;

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
    bridge: Res<RenderFrameWorkBridge>,
    mut samples: ResMut<RenderFrameWorkSamples>,
) {
    let packed_sample = bridge.0.load(Ordering::Acquire);
    samples.last_sequence = (packed_sample >> RENDER_WORK_MICROS_BITS) as u32;
    samples.skipped_samples = 0;
    samples.micros.clear();
}

pub(super) fn log_render_frame_work(mut samples: ResMut<RenderFrameWorkSamples>) {
    let diagnostic = samples.take_diagnostic();
    let line = format!(
        "render work: samples={} skipped_samples={} avg_us={} p50_us={} p95_us={} p99_us={} max_us={}",
        diagnostic.count,
        diagnostic.skipped_samples,
        diagnostic.average_micros,
        diagnostic.p50_micros,
        diagnostic.p95_micros,
        diagnostic.p99_micros,
        diagnostic.max_micros,
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