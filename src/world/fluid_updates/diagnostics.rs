use std::time::Duration;

use bevy::prelude::*;

use super::{
    solver::FluidSolverMetrics,
    state::{PendingFluidBacklog, PendingFluidUpdates},
};

const FLUID_DIAGNOSTIC_INTERVAL_SECONDS: f32 = 10.0;

#[derive(Default)]
pub(in crate::world) struct FluidPerformanceDiagnostics {
    timer: Option<Timer>,
    totals: FluidSolverMetrics,
    active_frames: u64,
}

impl FluidPerformanceDiagnostics {
    pub(super) fn record(
        &mut self,
        delta: Duration,
        metrics: FluidSolverMetrics,
        pending: &PendingFluidUpdates,
        catch_up: bool,
    ) {
        if metrics.desired_evaluations > 0 {
            self.active_frames = self.active_frames.saturating_add(1);
        }
        self.totals.accumulate(metrics);

        let timer = self.timer.get_or_insert_with(|| {
            Timer::from_seconds(FLUID_DIAGNOSTIC_INTERVAL_SECONDS, TimerMode::Repeating)
        });
        timer.tick(delta);
        if !timer.just_finished() || self.totals.desired_evaluations == 0 {
            return;
        }

        let searches_per_desired =
            self.totals.downhill_searches as f64 / self.totals.desired_evaluations as f64;
        let nodes_per_search = if self.totals.downhill_searches == 0 {
            0.0
        } else {
            self.totals.downhill_nodes as f64 / self.totals.downhill_searches as f64
        };

        let PendingFluidBacklog {
            topology,
            wakes,
            scheduled,
            dormant_chunks,
        } = pending.backlog();

        info!(
            "fluid solver diagnostics: desired={} horizontal_candidates={} downhill_searches={} downhill_nodes={} searches_per_desired={searches_per_desired:.3} nodes_per_search={nodes_per_search:.2} active_frames={} topology_backlog={} wake_backlog={} scheduled_backlog={} dormant_chunks={} catch_up={catch_up}",
            self.totals.desired_evaluations,
            self.totals.horizontal_candidates,
            self.totals.downhill_searches,
            self.totals.downhill_nodes,
            self.active_frames,
            topology,
            wakes,
            scheduled,
            dormant_chunks,
        );

        self.totals = FluidSolverMetrics::default();
        self.active_frames = 0;
    }
}

