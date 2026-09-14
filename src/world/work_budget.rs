use std::time::{Duration, Instant};

pub(super) struct FrameWorkBudget {
    started: Instant,
    duration: Duration,
    minimum_items: usize,
    maximum_items: Option<usize>,
    processed: usize,
}

impl FrameWorkBudget {
    pub(super) fn new(duration: Duration, minimum_items: usize) -> Self {
        Self {
            started: Instant::now(),
            duration,
            minimum_items,
            maximum_items: None,
            processed: 0,
        }
    }

    pub(super) fn with_maximum_items(mut self, maximum_items: usize) -> Self {
        self.maximum_items = Some(maximum_items);
        self
    }

    pub(super) fn exhausted(&self) -> bool {
        self.maximum_items
            .is_some_and(|maximum| self.processed >= maximum)
            || (self.processed >= self.minimum_items && self.started.elapsed() >= self.duration)
    }

    pub(super) fn record(&mut self, items: usize) {
        self.processed = self.processed.saturating_add(items);
    }

    pub(super) fn processed(&self) -> usize {
        self.processed
    }
}
