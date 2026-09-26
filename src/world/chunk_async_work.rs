use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Instant,
};

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

const LOADING_QUEUE_DEPTH_PER_WORKER: usize = 4;

#[derive(Resource, Clone, Default)]
pub(crate) struct ChunkAsyncWorkLimiter {
    in_flight: Arc<AtomicUsize>,
    adaptive_limit: Arc<AtomicUsize>,
    metrics: Arc<ChunkAsyncWorkMetrics>,
}

#[derive(Default)]
struct ChunkAsyncWorkMetrics {
    generation: ChunkAsyncStageMetrics,
    initial_mesh: ChunkAsyncStageMetrics,
    remesh: ChunkAsyncStageMetrics,
}

#[derive(Default)]
struct ChunkAsyncStageMetrics {
    count: AtomicU64,
    total_nanos: AtomicU64,
    max_nanos: AtomicU64,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ChunkAsyncStageDiagnostic {
    pub(crate) count: u64,
    pub(crate) average_micros: u64,
    pub(crate) max_micros: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ChunkAsyncWorkDiagnostics {
    pub(crate) generation: ChunkAsyncStageDiagnostic,
    pub(crate) initial_mesh: ChunkAsyncStageDiagnostic,
    pub(crate) remesh: ChunkAsyncStageDiagnostic,
}

#[derive(Clone, Copy)]
enum ChunkAsyncStage {
    Generation,
    InitialMesh,
    Remesh,
}

impl ChunkAsyncWorkLimiter {
    pub(crate) fn try_acquire_generation(&self) -> Option<ChunkAsyncWorkPermit> {
        self.try_acquire_with_limit(self.limit(), ChunkAsyncStage::Generation)
    }

    pub(crate) fn try_acquire_loading_generation(&self) -> Option<ChunkAsyncWorkPermit> {
        self.try_acquire_with_limit(self.loading_queue_limit(), ChunkAsyncStage::Generation)
    }

    pub(crate) fn try_acquire_initial_mesh(&self) -> Option<ChunkAsyncWorkPermit> {
        self.try_acquire_with_limit(self.limit(), ChunkAsyncStage::InitialMesh)
    }

    pub(crate) fn try_acquire_loading_initial_mesh(&self) -> Option<ChunkAsyncWorkPermit> {
        self.try_acquire_with_limit(self.loading_queue_limit(), ChunkAsyncStage::InitialMesh)
    }

    pub(crate) fn loading_queue_limit(&self) -> usize {
        loading_queue_limit_for_workers(AsyncComputeTaskPool::get().thread_num().max(1))
    }

    pub(crate) fn try_acquire_remesh(&self) -> Option<ChunkAsyncWorkPermit> {
        self.try_acquire_with_limit(self.limit(), ChunkAsyncStage::Remesh)
    }

    fn try_acquire_with_limit(
        &self,
        limit: usize,
        stage: ChunkAsyncStage,
    ) -> Option<ChunkAsyncWorkPermit> {
        let mut current = self.in_flight.load(Ordering::Acquire);

        loop {
            if current >= limit {
                return None;
            }

            match self.in_flight.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Some(ChunkAsyncWorkPermit {
                        in_flight: Arc::clone(&self.in_flight),
                        metrics: Arc::clone(&self.metrics),
                        stage,
                        started: Instant::now(),
                    });
                }
                Err(observed) => current = observed,
            }
        }
    }

    pub(crate) fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Acquire)
    }

    pub(crate) fn base_limit(&self) -> usize {
        base_async_limit_for_workers(AsyncComputeTaskPool::get().thread_num().max(1))
    }

    pub(crate) fn limit(&self) -> usize {
        let base = self.base_limit();
        let minimum = adaptive_limit_floor(base);
        let adaptive = self.adaptive_limit.load(Ordering::Acquire);
        if adaptive == 0 {
            base
        } else {
            adaptive.clamp(minimum, base)
        }
    }

    pub(crate) fn reset_adaptive_limit(&self) {
        self.adaptive_limit.store(0, Ordering::Release);
    }

    fn set_adaptive_limit(&self, limit: usize) {
        let base = self.base_limit();
        self.adaptive_limit.store(
            limit.clamp(adaptive_limit_floor(base), base),
            Ordering::Release,
        );
    }

    pub(crate) fn take_diagnostics(&self) -> ChunkAsyncWorkDiagnostics {
        ChunkAsyncWorkDiagnostics {
            generation: self.metrics.generation.take(),
            initial_mesh: self.metrics.initial_mesh.take(),
            remesh: self.metrics.remesh.take(),
        }
    }
}

impl ChunkAsyncStageMetrics {
    fn record(&self, elapsed_nanos: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.total_nanos
            .fetch_add(elapsed_nanos, Ordering::Relaxed);
        self.max_nanos.fetch_max(elapsed_nanos, Ordering::Relaxed);
    }

    fn take(&self) -> ChunkAsyncStageDiagnostic {
        let count = self.count.swap(0, Ordering::Relaxed);
        let total_nanos = self.total_nanos.swap(0, Ordering::Relaxed);
        let max_nanos = self.max_nanos.swap(0, Ordering::Relaxed);
        ChunkAsyncStageDiagnostic {
            count,
            average_micros: total_nanos.checked_div(count).unwrap_or(0) / 1_000,
            max_micros: max_nanos / 1_000,
        }
    }
}

// Adapt against sustained frame pressure, not individual hitches. The previous
// recovery threshold required >65 FPS, which is unreachable under a 60 Hz
// present cadence and permanently pinned the limiter to its floor after a few
// slow frames.
const ASYNC_SLOW_FRAME_SECONDS: f32 = 1.0 / 45.0;
const ASYNC_RECOVERY_FRAME_SECONDS: f32 = 1.0 / 55.0;
const ASYNC_SLOW_FRAMES: u16 = 8;
const ASYNC_RECOVERY_FRAMES: u16 = 30;

fn loading_queue_limit_for_workers(workers: usize) -> usize {
    workers.max(1).saturating_mul(LOADING_QUEUE_DEPTH_PER_WORKER)
}

fn base_async_limit_for_workers(workers: usize) -> usize {
    // Chunk generation/meshing is sustained CPU work. Keep roughly 40% of the
    // async pool free so renderer support work and unrelated async systems are
    // not starved while streaming is active.
    (workers.max(1) * 3).div_ceil(5).max(1)
}

fn adaptive_limit_floor(base: usize) -> usize {
    if base <= 2 {
        return base.max(1);
    }

    base.div_ceil(2).max(2)
}

#[derive(Default)]
pub(crate) struct ChunkAsyncAdaptationState {
    slow_frames: u16,
    recovery_frames: u16,
}

impl ChunkAsyncAdaptationState {
    fn observe(
        &mut self,
        frame_seconds: f32,
        current: usize,
        base: usize,
    ) -> Option<usize> {
        if frame_seconds > ASYNC_SLOW_FRAME_SECONDS {
            self.recovery_frames = 0;
            self.slow_frames = self.slow_frames.saturating_add(1);
            if self.slow_frames >= ASYNC_SLOW_FRAMES
                && current > adaptive_limit_floor(base)
            {
                self.slow_frames = 0;
                return Some(current - 1);
            }
            return None;
        }

        self.slow_frames = 0;
        if frame_seconds <= ASYNC_RECOVERY_FRAME_SECONDS && current < base {
            self.recovery_frames = self.recovery_frames.saturating_add(1);
            if self.recovery_frames >= ASYNC_RECOVERY_FRAMES {
                self.recovery_frames = 0;
                return Some(current + 1);
            }
        } else {
            self.recovery_frames = 0;
        }

        None
    }
}

pub(crate) fn reset_chunk_async_work_limit(
    limiter: Res<ChunkAsyncWorkLimiter>,
) {
    limiter.reset_adaptive_limit();
}

pub(crate) fn tune_chunk_async_work(
    time: Res<Time<Real>>,
    limiter: Res<ChunkAsyncWorkLimiter>,
    mut state: Local<ChunkAsyncAdaptationState>,
) {
    let frame_seconds = time.delta_secs();
    let current = limiter.limit();
    let base = limiter.base_limit();

    if let Some(next) = state.observe(frame_seconds, current, base) {
        limiter.set_adaptive_limit(next);
    }
}

pub(crate) struct ChunkAsyncWorkPermit {
    in_flight: Arc<AtomicUsize>,
    metrics: Arc<ChunkAsyncWorkMetrics>,
    stage: ChunkAsyncStage,
    started: Instant,
}

impl Drop for ChunkAsyncWorkPermit {
    fn drop(&mut self) {
        let elapsed_nanos = self
            .started
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;
        match self.stage {
            ChunkAsyncStage::Generation => self.metrics.generation.record(elapsed_nanos),
            ChunkAsyncStage::InitialMesh => self.metrics.initial_mesh.record(elapsed_nanos),
            ChunkAsyncStage::Remesh => self.metrics.remesh.record(elapsed_nanos),
        }

        let previous = self.in_flight.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "chunk async work limiter cannot underflow");
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ASYNC_RECOVERY_FRAMES, ASYNC_SLOW_FRAMES, ChunkAsyncAdaptationState,
        adaptive_limit_floor, base_async_limit_for_workers, loading_queue_limit_for_workers,
    };

    #[test]
    fn loading_queue_keeps_multiple_batches_ready_per_worker() {
        assert_eq!(loading_queue_limit_for_workers(1), 4);
        assert_eq!(loading_queue_limit_for_workers(4), 16);
        assert_eq!(loading_queue_limit_for_workers(8), 32);
    }

    #[test]
    fn base_limit_reserves_headroom_for_non_chunk_work() {
        assert_eq!(base_async_limit_for_workers(1), 1);
        assert_eq!(base_async_limit_for_workers(2), 2);
        assert_eq!(base_async_limit_for_workers(4), 3);
        assert_eq!(base_async_limit_for_workers(8), 5);
        assert_eq!(base_async_limit_for_workers(12), 8);
    }

    #[test]
    fn adaptive_floor_preserves_half_of_chunk_throughput_on_wide_pools() {
        assert_eq!(adaptive_limit_floor(1), 1);
        assert_eq!(adaptive_limit_floor(2), 2);
        assert_eq!(adaptive_limit_floor(3), 2);
        assert_eq!(adaptive_limit_floor(4), 2);
        assert_eq!(adaptive_limit_floor(6), 3);
        assert_eq!(adaptive_limit_floor(12), 6);
    }

    #[test]
    fn sixty_hz_frames_can_recover_the_async_limit() {
        let mut state = ChunkAsyncAdaptationState::default();
        let base = 4;
        let mut current = 2;

        for _ in 0..ASYNC_RECOVERY_FRAMES {
            if let Some(next) = state.observe(1.0 / 60.0, current, base) {
                current = next;
            }
        }

        assert_eq!(current, 3);
    }

    #[test]
    fn isolated_hitches_do_not_reduce_async_capacity() {
        let mut state = ChunkAsyncAdaptationState::default();
        let base = 4;
        let current = 4;

        assert_eq!(state.observe(0.100, current, base), None);
        assert_eq!(state.observe(1.0 / 60.0, current, base), None);

        for _ in 0..ASYNC_SLOW_FRAMES - 1 {
            assert_eq!(state.observe(1.0 / 30.0, current, base), None);
        }
    }

    #[test]
    fn sustained_slow_frames_still_back_off() {
        let mut state = ChunkAsyncAdaptationState::default();
        let base = 4;
        let current = 4;
        let mut next = None;

        for _ in 0..ASYNC_SLOW_FRAMES {
            next = state.observe(1.0 / 30.0, current, base);
        }

        assert_eq!(next, Some(3));
    }
}
