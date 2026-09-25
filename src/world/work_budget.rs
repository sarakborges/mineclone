use std::time::{Duration, Instant};

use bevy::prelude::{Res, ResMut, Resource, Time};

const WORLD_WORK_BUDGET_PRESSURED: Duration = Duration::from_millis(2);
const WORLD_WORK_BUDGET_NORMAL: Duration = Duration::from_millis(3);
const WORLD_WORK_BUDGET_FAST: Duration = Duration::from_millis(4);
const WORLD_WORK_PRESSURE_FRAME_SECONDS: f32 = 1.0 / 60.0;
const WORLD_WORK_FAST_FRAME_SECONDS: f32 = 1.0 / 75.0;

#[derive(Resource)]
pub(super) struct WorldFrameWorkBudget {
    deadline: Instant,
}

impl Default for WorldFrameWorkBudget {
    fn default() -> Self {
        Self {
            deadline: Instant::now() + WORLD_WORK_BUDGET_NORMAL,
        }
    }
}

impl WorldFrameWorkBudget {
    pub(super) fn deadline(&self) -> Instant {
        self.deadline
    }

    fn reset(&mut self, duration: Duration) {
        self.deadline = Instant::now() + duration;
    }
}

fn gameplay_world_work_budget(frame_seconds: f32) -> Duration {
    if frame_seconds > WORLD_WORK_PRESSURE_FRAME_SECONDS {
        WORLD_WORK_BUDGET_PRESSURED
    } else if frame_seconds > WORLD_WORK_FAST_FRAME_SECONDS {
        WORLD_WORK_BUDGET_NORMAL
    } else {
        WORLD_WORK_BUDGET_FAST
    }
}

pub(super) fn begin_world_frame_work_budget(
    time: Res<Time<Real>>,
    mut budget: ResMut<WorldFrameWorkBudget>,
) {
    budget.reset(gameplay_world_work_budget(time.delta_secs()));
}

pub(super) struct FrameWorkBudget {
    started: Instant,
    duration: Duration,
    minimum_items: usize,
    maximum_items: Option<usize>,
    global_deadline: Option<Instant>,
    processed: usize,
}

impl FrameWorkBudget {
    pub(super) fn new(duration: Duration, minimum_items: usize) -> Self {
        Self {
            started: Instant::now(),
            duration,
            minimum_items,
            maximum_items: None,
            global_deadline: None,
            processed: 0,
        }
    }

    pub(super) fn with_global_deadline(mut self, deadline: Instant) -> Self {
        self.global_deadline = Some(deadline);
        self
    }

    pub(super) fn with_maximum_items(mut self, maximum_items: usize) -> Self {
        self.maximum_items = Some(maximum_items);
        self
    }

    pub(super) fn exhausted(&self) -> bool {
        self.maximum_items
            .is_some_and(|maximum| self.processed >= maximum)
            || (self.processed >= self.minimum_items
                && (self.started.elapsed() >= self.duration
                    || self
                        .global_deadline
                        .is_some_and(|deadline| Instant::now() >= deadline)))
    }

    pub(super) fn record(&mut self, items: usize) {
        self.processed = self.processed.saturating_add(items);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_work_budget_shrinks_before_frame_rate_drops_below_sixty() {
        assert_eq!(
            gameplay_world_work_budget(1.0 / 90.0),
            WORLD_WORK_BUDGET_FAST
        );
        assert_eq!(
            gameplay_world_work_budget(1.0 / 70.0),
            WORLD_WORK_BUDGET_NORMAL
        );
        assert_eq!(
            gameplay_world_work_budget(1.0 / 55.0),
            WORLD_WORK_BUDGET_PRESSURED
        );
    }
}
