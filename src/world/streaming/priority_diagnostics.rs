use std::{
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
    time::Duration,
};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct StreamingPriorityScanDiagnostic {
    pub(super) count: u64,
    pub(super) average_micros: u64,
    pub(super) max_micros: u64,
    pub(super) max_queue_len: usize,
}

#[derive(Default)]
struct StreamingPriorityScanMetrics {
    count: AtomicU64,
    total_nanos: AtomicU64,
    max_nanos: AtomicU64,
    max_queue_len: AtomicUsize,
}

impl StreamingPriorityScanMetrics {
    fn record(&self, elapsed: Duration, queue_len: usize) {
        let elapsed_nanos = elapsed.as_nanos().min(u128::from(u64::MAX)) as u64;
        self.count.fetch_add(1, Ordering::Relaxed);
        self.total_nanos
            .fetch_add(elapsed_nanos, Ordering::Relaxed);
        self.max_nanos.fetch_max(elapsed_nanos, Ordering::Relaxed);
        self.max_queue_len.fetch_max(queue_len, Ordering::Relaxed);
    }

    fn take(&self) -> StreamingPriorityScanDiagnostic {
        let count = self.count.swap(0, Ordering::Relaxed);
        let total_nanos = self.total_nanos.swap(0, Ordering::Relaxed);
        StreamingPriorityScanDiagnostic {
            count,
            average_micros: total_nanos.checked_div(count).unwrap_or(0) / 1_000,
            max_micros: self.max_nanos.swap(0, Ordering::Relaxed) / 1_000,
            max_queue_len: self.max_queue_len.swap(0, Ordering::Relaxed),
        }
    }
}

#[derive(Default)]
pub(super) struct StreamingPriorityDiagnostics {
    pending: StreamingPriorityScanMetrics,
    ready: StreamingPriorityScanMetrics,
}

impl StreamingPriorityDiagnostics {
    pub(super) fn record_pending(&self, scan: Option<(Duration, usize)>) {
        if let Some((elapsed, queue_len)) = scan {
            self.pending.record(elapsed, queue_len);
        }
    }

    pub(super) fn record_ready(&self, scan: Option<(Duration, usize)>) {
        if let Some((elapsed, queue_len)) = scan {
            self.ready.record(elapsed, queue_len);
        }
    }

    pub(super) fn take(
        &self,
    ) -> (
        StreamingPriorityScanDiagnostic,
        StreamingPriorityScanDiagnostic,
    ) {
        (self.pending.take(), self.ready.take())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_are_owned_and_drained_independently() {
        let diagnostics = StreamingPriorityDiagnostics::default();
        diagnostics.record_pending(Some((Duration::from_micros(15), 7)));
        diagnostics.record_ready(Some((Duration::from_micros(9), 3)));
        diagnostics.record_pending(None);

        let (pending, ready) = diagnostics.take();
        assert_eq!(pending.count, 1);
        assert_eq!(pending.average_micros, 15);
        assert_eq!(pending.max_micros, 15);
        assert_eq!(pending.max_queue_len, 7);
        assert_eq!(ready.count, 1);
        assert_eq!(ready.average_micros, 9);
        assert_eq!(ready.max_micros, 9);
        assert_eq!(ready.max_queue_len, 3);

        let (pending, ready) = diagnostics.take();
        assert_eq!(pending.count, 0);
        assert_eq!(ready.count, 0);
    }
}
