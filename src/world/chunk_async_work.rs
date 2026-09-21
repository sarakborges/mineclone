use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

#[derive(Resource, Clone, Default)]
pub(crate) struct ChunkAsyncWorkLimiter {
    in_flight: Arc<AtomicUsize>,
}

impl ChunkAsyncWorkLimiter {
    pub(crate) fn try_acquire_generation(&self) -> Option<ChunkAsyncWorkPermit> {
        let limit = self.limit();
        let generation_limit = if limit > 1 { limit - 1 } else { 1 };
        self.try_acquire_with_limit(generation_limit)
    }

    pub(crate) fn try_acquire_render(&self) -> Option<ChunkAsyncWorkPermit> {
        self.try_acquire_with_limit(self.limit())
    }

    fn try_acquire_with_limit(&self, limit: usize) -> Option<ChunkAsyncWorkPermit> {
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
                    });
                }
                Err(observed) => current = observed,
            }
        }
    }

    pub(crate) fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Acquire)
    }

    pub(crate) fn limit(&self) -> usize {
        let workers = AsyncComputeTaskPool::get().thread_num().max(1);
        // Keep some executor headroom instead of allowing independent chunk
        // subsystems to saturate every worker with long CPU-heavy jobs.
        (workers * 3).div_ceil(4).max(1)
    }
}

pub(crate) struct ChunkAsyncWorkPermit {
    in_flight: Arc<AtomicUsize>,
}

impl Drop for ChunkAsyncWorkPermit {
    fn drop(&mut self) {
        let previous = self.in_flight.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "chunk async work limiter cannot underflow");
    }
}
