use std::{collections::VecDeque, time::Instant};

use bevy::prelude::*;

use crate::app::crash_log::append_runtime_diagnostic;

const MAIN_WORLD_SAMPLE_CAPACITY: usize = 4096;

#[derive(Clone, Copy, Debug, Default)]
struct StageTimingDiagnostic {
    count: usize,
    average_micros: u64,
    p95_micros: u64,
    p99_micros: u64,
    max_micros: u64,
}

#[derive(Default)]
struct StageTimingSamples {
    started_at: Option<Instant>,
    micros: VecDeque<u64>,
}

impl StageTimingSamples {
    fn begin(&mut self) {
        self.started_at = Some(Instant::now());
    }

    fn finish(&mut self) {
        let Some(started_at) = self.started_at.take() else {
            return;
        };
        self.record(started_at.elapsed());
    }

    fn record(&mut self, elapsed: std::time::Duration) {
        if self.micros.len() >= MAIN_WORLD_SAMPLE_CAPACITY {
            self.micros.pop_front();
        }
        self.micros
            .push_back(elapsed.as_micros().min(u128::from(u64::MAX)) as u64);
    }

    fn take_diagnostic(&mut self) -> StageTimingDiagnostic {
        if self.micros.is_empty() {
            return StageTimingDiagnostic::default();
        }

        let mut values = self.micros.drain(..).collect::<Vec<_>>();
        values.sort_unstable();
        let count = values.len();
        let total = values
            .iter()
            .fold(0_u128, |sum, value| sum + u128::from(*value));

        StageTimingDiagnostic {
            count,
            average_micros: (total / count as u128).min(u128::from(u64::MAX)) as u64,
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

#[derive(Resource, Default)]
pub(super) struct MainWorldWorkSamples {
    streaming: StageTimingSamples,
    retirement: StageTimingSamples,
    fluid: StageTimingSamples,
    lighting: StageTimingSamples,
    remesh: StageTimingSamples,
    residency: StageTimingSamples,
    visibility: StageTimingSamples,
    generation_refill: StageTimingSamples,
    deferred_mesh_retirement: StageTimingSamples,
}

macro_rules! stage_markers {
    ($begin:ident, $finish:ident, $field:ident) => {
        pub(super) fn $begin(mut samples: ResMut<MainWorldWorkSamples>) {
            samples.$field.begin();
        }

        pub(super) fn $finish(mut samples: ResMut<MainWorldWorkSamples>) {
            samples.$field.finish();
        }
    };
}

stage_markers!(begin_streaming_work, finish_streaming_work, streaming);
stage_markers!(begin_retirement_work, finish_retirement_work, retirement);
stage_markers!(begin_fluid_work, finish_fluid_work, fluid);
stage_markers!(begin_lighting_work, finish_lighting_work, lighting);
stage_markers!(begin_remesh_work, finish_remesh_work, remesh);
stage_markers!(begin_residency_work, finish_residency_work, residency);
stage_markers!(begin_visibility_work, finish_visibility_work, visibility);
stage_markers!(
    begin_generation_refill_work,
    finish_generation_refill_work,
    generation_refill
);
stage_markers!(
    begin_deferred_mesh_retirement_work,
    finish_deferred_mesh_retirement_work,
    deferred_mesh_retirement
);

pub(super) fn log_main_world_work(mut samples: ResMut<MainWorldWorkSamples>) {
    let streaming = samples.streaming.take_diagnostic();
    let retirement = samples.retirement.take_diagnostic();
    let fluid = samples.fluid.take_diagnostic();
    let lighting = samples.lighting.take_diagnostic();
    let remesh = samples.remesh.take_diagnostic();
    let residency = samples.residency.take_diagnostic();
    let visibility = samples.visibility.take_diagnostic();
    let generation_refill = samples.generation_refill.take_diagnostic();
    let deferred_mesh_retirement = samples.deferred_mesh_retirement.take_diagnostic();

    let diagnostic = format!(
        "main world stages: streaming_count={} streaming_avg_us={} streaming_p95_us={} streaming_p99_us={} streaming_max_us={} retirement_count={} retirement_avg_us={} retirement_p95_us={} retirement_p99_us={} retirement_max_us={} fluid_count={} fluid_avg_us={} fluid_p95_us={} fluid_p99_us={} fluid_max_us={} lighting_count={} lighting_avg_us={} lighting_p95_us={} lighting_p99_us={} lighting_max_us={} remesh_count={} remesh_avg_us={} remesh_p95_us={} remesh_p99_us={} remesh_max_us={} residency_count={} residency_avg_us={} residency_p95_us={} residency_p99_us={} residency_max_us={} visibility_count={} visibility_avg_us={} visibility_p95_us={} visibility_p99_us={} visibility_max_us={} generation_refill_count={} generation_refill_avg_us={} generation_refill_p95_us={} generation_refill_p99_us={} generation_refill_max_us={} deferred_mesh_retirement_count={} deferred_mesh_retirement_avg_us={} deferred_mesh_retirement_p95_us={} deferred_mesh_retirement_p99_us={} deferred_mesh_retirement_max_us={}",
        streaming.count,
        streaming.average_micros,
        streaming.p95_micros,
        streaming.p99_micros,
        streaming.max_micros,
        retirement.count,
        retirement.average_micros,
        retirement.p95_micros,
        retirement.p99_micros,
        retirement.max_micros,
        fluid.count,
        fluid.average_micros,
        fluid.p95_micros,
        fluid.p99_micros,
        fluid.max_micros,
        lighting.count,
        lighting.average_micros,
        lighting.p95_micros,
        lighting.p99_micros,
        lighting.max_micros,
        remesh.count,
        remesh.average_micros,
        remesh.p95_micros,
        remesh.p99_micros,
        remesh.max_micros,
        residency.count,
        residency.average_micros,
        residency.p95_micros,
        residency.p99_micros,
        residency.max_micros,
        visibility.count,
        visibility.average_micros,
        visibility.p95_micros,
        visibility.p99_micros,
        visibility.max_micros,
        generation_refill.count,
        generation_refill.average_micros,
        generation_refill.p95_micros,
        generation_refill.p99_micros,
        generation_refill.max_micros,
        deferred_mesh_retirement.count,
        deferred_mesh_retirement.average_micros,
        deferred_mesh_retirement.p95_micros,
        deferred_mesh_retirement.p99_micros,
        deferred_mesh_retirement.max_micros,
    );

    info!("{diagnostic}");
    let _ = append_runtime_diagnostic(&diagnostic);
}
