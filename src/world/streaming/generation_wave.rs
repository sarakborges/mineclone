use bevy::{platform::collections::HashSet, prelude::IVec3};

use crate::{
    voxel::deduplicated_queue::DeduplicatedQueue,
    world::fluid_updates::GeneratedFluidSettling,
};

/// Owns the lifecycle of one generation publication wave independently from
/// chunk selection and presentation queues. `GeneratedFluidSettling` remains
/// its own simulation implementation; this state only owns the fact that an
/// active generation wave may currently be waiting on that solver.
#[derive(Default)]
pub(super) struct GenerationWaveState {
    targets: HashSet<IVec3>,
    pending: DeduplicatedQueue<IVec3>,
    prefetch_targets: HashSet<IVec3>,
    staged_generated_chunks: HashSet<IVec3>,
    settled_publication_chunks: Vec<IVec3>,
    pub(super) fluid_settling: GeneratedFluidSettling,
}

impl GenerationWaveState {
    pub(super) fn start_target(&mut self, coord: IVec3) {
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
            && (self.pending.len() > 0
                || (pending_request_count > 0 && self.accepts_new_targets()))
    }

    pub(super) fn stage_generated_chunk(&mut self, coord: IVec3) {
        debug_assert!(
            self.targets.contains(&coord),
            "only an active generation-wave target may become staged"
        );
        self.staged_generated_chunks.insert(coord);
    }

    pub(super) fn abandon_target(&mut self, coord: IVec3) {
        self.pending.remove(coord);
        self.targets.remove(&coord);
        self.prefetch_targets.remove(&coord);
    }

    pub(super) fn mark_prefetched(&mut self, coord: IVec3) {
        debug_assert!(
            !self.targets.contains(&coord),
            "prefetched generation cannot already belong to the active wave"
        );
        self.prefetch_targets.insert(coord);
    }

    pub(super) fn complete_target(&mut self, coord: IVec3) {
        debug_assert!(
            !self.staged_generated_chunks.contains(&coord),
            "completed generation-wave target cannot remain staged"
        );
        debug_assert!(
            !self.fluid_settling.contains(coord),
            "completed generation-wave target cannot remain settling-owned"
        );
        let removed = self.targets.remove(&coord);
        debug_assert!(
            removed,
            "completed generation-wave target must still own its reservation: {coord:?}"
        );
    }

    pub(super) fn take_staged_generated_chunks(&mut self) -> Vec<IVec3> {
        let mut staged = self.staged_generated_chunks.drain().collect::<Vec<_>>();
        staged.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        staged
    }

    pub(super) fn begin_settled_publication(&mut self, mut chunks: Vec<IVec3>) {
        debug_assert!(
            self.settled_publication_chunks.is_empty(),
            "settled publication queue must be empty before a new wave is staged"
        );
        chunks.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        self.settled_publication_chunks = chunks;
    }

    pub(super) fn has_settled_publication(&self) -> bool {
        !self.settled_publication_chunks.is_empty()
    }

    pub(super) fn pop_settled_publication_chunk(&mut self) -> Option<IVec3> {
        self.settled_publication_chunks.pop()
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
        self.targets.contains(&coord)
            || self.prefetch_targets.contains(&coord)
            || self.staged_generated_chunks.contains(&coord)
            || self.fluid_settling.contains(coord)
    }

    pub(super) fn resident_generated_chunk_is_unpublished(&self, coord: IVec3) -> bool {
        self.staged_generated_chunks.contains(&coord)
            || self.settled_publication_chunks.contains(&coord)
            || self.fluid_settling.contains(coord)
    }

    pub(super) fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub(super) fn pop_pending(&mut self) -> Option<IVec3> {
        self.pending.pop()
    }

    pub(super) fn enqueue_pending(&mut self, coord: IVec3) {
        self.pending.enqueue(coord);
    }

    pub(super) fn target_len(&self) -> usize {
        self.targets.len()
    }

    pub(super) fn contains_target(&self, coord: IVec3) -> bool {
        self.targets.contains(&coord)
    }

    pub(super) fn prefetch_len(&self) -> usize {
        self.prefetch_targets.len()
    }

    pub(super) fn contains_prefetch(&self, coord: IVec3) -> bool {
        self.prefetch_targets.contains(&coord)
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
        self.targets.iter().copied()
    }

    pub(super) fn settling_or_publishing(&self) -> bool {
        self.fluid_settling.is_active() || !self.settled_publication_chunks.is_empty()
    }
}
