use std::time::{Duration, Instant};

use bevy::prelude::{ResMut, Resource};

const GAMEPLAY_WORLD_WORK_BUDGET: Duration = Duration::from_millis(8);

#[derive(Resource)]
pub(super) struct WorldFrameWorkBudget {
    deadline: Instant,
}

impl Default for WorldFrameWorkBudget {
    fn default() -> Self {
        Self {
            deadline: Instant::now() + GAMEPLAY_WORLD_WORK_BUDGET,
        }
    }
}

impl WorldFrameWorkBudget {
    pub(super) fn deadline(&self) -> Instant {
        self.deadline
    }

    fn reset(&mut self) {
        self.deadline = Instant::now() + GAMEPLAY_WORLD_WORK_BUDGET;
    }
}

pub(super) fn begin_world_frame_work_budget(mut budget: ResMut<WorldFrameWorkBudget>) {
    budget.reset();
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
