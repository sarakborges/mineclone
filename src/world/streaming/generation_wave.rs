use bevy::{platform::collections::HashSet, prelude::IVec3};

use super::DeduplicatedQueue;
use crate::{voxel::coordinates::ChunkCoord, world::fluid_updates::GeneratedFluidSettling};

/// Owns the lifecycle of one generation publication wave independently from
/// chunk selection and presentation queues. `GeneratedFluidSettling` remains
/// its own simulation implementation; this state only owns the fact that an
/// active generation wave may currently be waiting on that solver.
#[derive(Default)]
pub(super) struct GenerationWaveState {
    targets: HashSet<ChunkCoord>,
    pending: DeduplicatedQueue<ChunkCoord>,
    prefetch_targets: HashSet<ChunkCoord>,
    staged_generated_chunks: HashSet<ChunkCoord>,
    settled_publication_chunks: Vec<ChunkCoord>,
    pub(super) fluid_settling: GeneratedFluidSettling,
}

impl GenerationWaveState {
    pub(super) fn start_target(&mut self, coord: IVec3) {
        let coord = ChunkCoord::from_ivec3(coord);
        if self.targets.insert(coord) {
            self.pending.enqueue(coord);
        }
    }

    pub(super) fn is_active(&self) -> bool {
        !self.targets.is_empty()
            || !self.staged_generated_chunks.is_empty()
            || !self.settled_publication_chunks.is_empty()
            || self.fluid_settling.is_active()
    }

    pub(super) fn accepts_new_targets(&self) -> bool {
        !self.fluid_settling.is_active()
            && self.staged_generated_chunks.is_empty()
            && self.settled_publication_chunks.is_empty()
    }

    pub(super) fn dispatch_work_exists(&self, pending_request_count: usize) -> bool {
        !self.fluid_settling.is_active()
            && self.settled_publication_chunks.is_empty()
            && (self.pending.len() > 0 || (pending_request_count > 0 && self.accepts_new_targets()))
    }

    pub(super) fn stage_generated_chunk(&mut self, coord: IVec3) {
        let coord = ChunkCoord::from_ivec3(coord);
        debug_assert!(
            self.targets.contains(&coord),
            "only an active generation-wave target may become staged"
        );
        self.staged_generated_chunks.insert(coord);
    }

    pub(super) fn abandon_target(&mut self, coord: IVec3) {
        let chunk_coord = ChunkCoord::from_ivec3(coord);
        assert!(
            !self.staged_generated_chunks.contains(&chunk_coord)
                && !self.settled_publication_chunks.contains(&chunk_coord)
                && !self.fluid_settling.contains(coord),
            "generation target cannot be abandoned after world truth enters staged/settling/publication ownership: {coord:?}"
        );
        self.pending.remove(chunk_coord);
        self.targets.remove(&chunk_coord);
        self.prefetch_targets.remove(&chunk_coord);
    }

    pub(super) fn mark_prefetched(&mut self, coord: IVec3) {
        let coord = ChunkCoord::from_ivec3(coord);
        debug_assert!(
            !self.targets.contains(&coord),
            "prefetched generation cannot already belong to the active wave"
        );
        self.prefetch_targets.insert(coord);
    }

    pub(super) fn complete_target(&mut self, coord: IVec3) {
        let chunk_coord = ChunkCoord::from_ivec3(coord);
        debug_assert!(
            !self.staged_generated_chunks.contains(&chunk_coord),
            "completed generation-wave target cannot remain staged"
        );
        debug_assert!(
            !self.fluid_settling.contains(coord),
            "completed generation-wave target cannot remain settling-owned"
        );
        let removed = self.targets.remove(&chunk_coord);
        debug_assert!(
            removed,
            "completed generation-wave target must still own its reservation: {coord:?}"
        );
    }

    pub(super) fn take_staged_generated_chunks(&mut self) -> Vec<IVec3> {
        let mut staged = self
            .staged_generated_chunks
            .drain()
            .map(ChunkCoord::as_ivec3)
            .collect::<Vec<_>>();
        staged.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        staged
    }

    pub(super) fn begin_settled_publication(&mut self, mut chunks: Vec<IVec3>) {
        debug_assert!(
            self.settled_publication_chunks.is_empty(),
            "settled publication queue must be empty before a new wave is staged"
        );
        chunks.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        self.settled_publication_chunks = chunks.into_iter().map(ChunkCoord::from_ivec3).collect();
    }

    pub(super) fn has_settled_publication(&self) -> bool {
        !self.settled_publication_chunks.is_empty()
    }

    pub(super) fn pop_settled_publication_chunk(&mut self) -> Option<IVec3> {
        self.settled_publication_chunks
            .pop()
            .map(ChunkCoord::as_ivec3)
    }

    pub(super) fn finish(&mut self) {
        assert!(
            self.staged_generated_chunks.is_empty(),
            "generation wave cannot finish with unpublished generated chunks"
        );
        assert!(
            self.settled_publication_chunks.is_empty(),
            "generation wave cannot finish with settled chunks awaiting publication"
        );
        assert!(
            self.pending.len() == 0,
            "generation wave cannot finish with unscheduled targets"
        );
        assert!(
            !self.fluid_settling.is_active(),
            "generation wave cannot finish while fluid settling is active"
        );
        assert!(
            self.targets.is_empty(),
            "generation wave cannot finish with unresolved target reservations: {:?}",
            self.targets
        );

        self.targets.extend(self.prefetch_targets.drain());
    }

    pub(super) fn contains_unpublished(&self, coord: IVec3) -> bool {
        let chunk_coord = ChunkCoord::from_ivec3(coord);
        self.targets.contains(&chunk_coord)
            || self.prefetch_targets.contains(&chunk_coord)
            || self.staged_generated_chunks.contains(&chunk_coord)
            || self.fluid_settling.contains(coord)
    }

    pub(super) fn resident_generated_chunk_is_unpublished(&self, coord: IVec3) -> bool {
        let chunk_coord = ChunkCoord::from_ivec3(coord);
        self.staged_generated_chunks.contains(&chunk_coord)
            || self.settled_publication_chunks.contains(&chunk_coord)
            || self.fluid_settling.contains(coord)
    }

    pub(super) fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub(super) fn pop_pending(&mut self) -> Option<IVec3> {
        self.pending.pop().map(ChunkCoord::as_ivec3)
    }

    pub(super) fn enqueue_pending(&mut self, coord: IVec3) {
        self.pending.enqueue(ChunkCoord::from_ivec3(coord));
    }

    pub(super) fn target_len(&self) -> usize {
        self.targets.len()
    }

    pub(super) fn contains_target(&self, coord: IVec3) -> bool {
        self.targets.contains(&ChunkCoord::from_ivec3(coord))
    }

    pub(super) fn prefetch_len(&self) -> usize {
        self.prefetch_targets.len()
    }

    pub(super) fn contains_prefetch(&self, coord: IVec3) -> bool {
        self.prefetch_targets
            .contains(&ChunkCoord::from_ivec3(coord))
    }

    pub(super) fn prefetch_count(&self) -> usize {
        self.prefetch_targets.len()
    }

    pub(super) fn diagnostic_counts(&self) -> (usize, usize, usize) {
        (
            self.pending.len(),
            self.targets.len(),
            self.staged_generated_chunks.len(),
        )
    }

    pub(super) fn targets(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.targets.iter().copied().map(ChunkCoord::as_ivec3)
    }

    pub(super) fn settling_or_publishing(&self) -> bool {
        self.fluid_settling.is_active() || !self.settled_publication_chunks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abandonment_clears_pending_and_prefetch_reservations() {
        let pending = IVec3::new(1, 2, 3);
        let prefetched = IVec3::new(-4, 1, 7);
        let mut wave = GenerationWaveState::default();
        wave.start_target(pending);
        wave.mark_prefetched(prefetched);

        wave.abandon_target(pending);
        wave.abandon_target(prefetched);

        assert_eq!(wave.pending_len(), 0);
        assert!(!wave.contains_target(pending));
        assert!(!wave.contains_prefetch(prefetched));
        assert!(!wave.contains_unpublished(pending));
        assert!(!wave.contains_unpublished(prefetched));
    }

    #[test]
    fn abandoned_prefetch_is_not_promoted_when_current_wave_finishes() {
        let retained = IVec3::new(4, 1, -3);
        let stale = IVec3::new(-9, 2, 8);
        let mut wave = GenerationWaveState::default();
        wave.mark_prefetched(retained);
        wave.mark_prefetched(stale);

        wave.abandon_target(stale);
        wave.finish();

        assert!(wave.contains_target(retained));
        assert!(wave.contains_unpublished(retained));
        assert!(!wave.contains_target(stale));
        assert!(!wave.contains_unpublished(stale));
        assert_eq!(wave.prefetch_count(), 0);
    }

    #[test]
    #[should_panic(
        expected = "generation target cannot be abandoned after world truth enters staged/settling/publication ownership"
    )]
    fn staged_world_truth_cannot_be_abandoned_as_async_work() {
        let coord = IVec3::new(3, 1, -2);
        let mut wave = GenerationWaveState::default();
        wave.start_target(coord);
        wave.stage_generated_chunk(coord);

        wave.abandon_target(coord);
    }
}
