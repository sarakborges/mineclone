use std::{collections::VecDeque, hash::Hash};

use bevy::platform::collections::{HashMap, hash_map::Entry};

const MIN_PENDING_BEFORE_COMPACTION: usize = 64;

#[derive(Debug)]
pub(crate) struct DeduplicatedQueue<T> {
    pending: VecDeque<(T, u64)>,
    queued: HashMap<T, u64>,
    next_generation: u64,
    revision: u64,
}

impl<T> Default for DeduplicatedQueue<T> {
    fn default() -> Self {
        Self {
            pending: VecDeque::new(),
            queued: HashMap::new(),
            next_generation: 0,
            revision: 0,
        }
    }
}

impl<T> From<Vec<T>> for DeduplicatedQueue<T>
where
    T: Copy + Eq + Hash,
{
    fn from(values: Vec<T>) -> Self {
        let capacity = values.len();
        let mut queue = Self {
            pending: VecDeque::with_capacity(capacity),
            queued: HashMap::with_capacity(capacity),
            next_generation: 0,
            revision: 0,
        };
        for value in values {
            queue.enqueue(value);
        }
        queue
    }
}

impl<T> DeduplicatedQueue<T>
where
    T: Copy + Eq + Hash,
{
    pub(crate) fn reserve(&mut self, additional: usize) {
        self.pending.reserve(additional);
        self.queued.reserve(additional);
    }

    pub(crate) fn clear(&mut self) {
        let had_queued_values = !self.queued.is_empty();
        self.pending.clear();
        self.queued.clear();
        if had_queued_values {
            self.bump_revision();
        }
    }

    pub(crate) fn enqueue(&mut self, value: T) -> bool {
        let Self {
            pending,
            queued,
            next_generation,
            revision,
        } = self;
        let Entry::Vacant(entry) = queued.entry(value) else {
            return false;
        };

        let generation = *next_generation;
        *next_generation = generation
            .checked_add(1)
            .expect("deduplicated queue generation exhausted");
        entry.insert(generation);
        pending.push_back((value, generation));
        *revision = (*revision)
            .checked_add(1)
            .expect("deduplicated queue revision exhausted");
        true
    }

    pub(crate) fn enqueue_front(&mut self, value: T) {
        let generation = self.next_generation();
        self.queued.insert(value, generation);
        self.pending.push_front((value, generation));
        self.bump_revision();
        self.compact_if_sparse();
    }

    pub(crate) fn contains(&self, value: T) -> bool {
        self.queued.contains_key(&value)
    }

    pub(crate) fn remove(&mut self, value: T) -> bool {
        if self.queued.remove(&value).is_none() {
            return false;
        }

        if self.queued.is_empty() {
            self.pending.clear();
        } else {
            self.compact_if_sparse();
        }
        self.bump_revision();
        true
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        let Self {
            pending,
            queued,
            revision,
            ..
        } = self;

        while let Some((value, generation)) = pending.pop_front() {
            let Entry::Occupied(entry) = queued.entry(value) else {
                continue;
            };
            if *entry.get() != generation {
                continue;
            }

            entry.remove();
            if queued.is_empty() {
                pending.clear();
            }
            *revision = (*revision)
                .checked_add(1)
                .expect("deduplicated queue revision exhausted");
            return Some(value);
        }

        debug_assert!(queued.is_empty(), "active queue entries must have pending records");
        None
    }

    pub(crate) fn pop_where(&mut self, mut predicate: impl FnMut(T) -> bool) -> Option<T> {
        self.compact_if_sparse();
        let index = self.pending.iter().position(|(value, generation)| {
            self.queued.get(value).copied() == Some(*generation) && predicate(*value)
        })?;
        Some(self.remove_active_index(index))
    }

    pub(crate) fn pop_min_by_key<K: Ord>(
        &mut self,
        key: impl FnMut(T) -> K,
    ) -> Option<T> {
        self.pop_min_where_by_key(|_| true, key)
    }

    pub(crate) fn pop_min_where_by_key<K: Ord>(
        &mut self,
        mut predicate: impl FnMut(T) -> bool,
        mut key: impl FnMut(T) -> K,
    ) -> Option<T> {
        self.compact_if_sparse();
        let mut best: Option<(usize, K)> = None;

        for (index, (value, generation)) in self.pending.iter().enumerate() {
            if self.queued.get(value).copied() != Some(*generation) || !predicate(*value) {
                continue;
            }

            let candidate_key = key(*value);
            if best
                .as_ref()
                .is_none_or(|(_, best_key)| candidate_key < *best_key)
            {
                best = Some((index, candidate_key));
            }
        }

        let (index, _) = best?;
        Some(self.remove_active_index(index))
    }

    pub(crate) fn len(&self) -> usize {
        self.queued.len()
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = T> + '_ {
        self.queued.keys().copied()
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    fn remove_active_index(&mut self, index: usize) -> T {
        let (value, generation) = self
            .pending
            .remove(index)
            .expect("located queue index must remain valid");
        debug_assert_eq!(self.queued.get(&value).copied(), Some(generation));
        self.queued.remove(&value);

        if self.queued.is_empty() {
            self.pending.clear();
        }
        self.bump_revision();
        value
    }

    fn next_generation(&mut self) -> u64 {
        let generation = self.next_generation;
        self.next_generation = self
            .next_generation
            .checked_add(1)
            .expect("deduplicated queue generation exhausted");
        generation
    }

    fn bump_revision(&mut self) {
        self.revision = self
            .revision
            .checked_add(1)
            .expect("deduplicated queue revision exhausted");
    }

    fn compact_if_sparse(&mut self) {
        let active = self.queued.len();
        let compact_threshold = active
            .saturating_mul(2)
            .saturating_add(MIN_PENDING_BEFORE_COMPACTION);
        if self.pending.len() <= compact_threshold {
            return;
        }

        self.pending.retain(|(value, generation)| {
            self.queued.get(value).copied() == Some(*generation)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_deduplicates_values() {
        let mut queue = DeduplicatedQueue::default();

        assert!(queue.enqueue(1));
        assert!(!queue.enqueue(1));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn enqueue_front_promotes_existing_values() {
        let mut queue = DeduplicatedQueue::default();
        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue_front(2);

        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(1));
    }

    #[test]
    fn repeated_priority_promotions_keep_one_active_value() {
        let mut queue = DeduplicatedQueue::default();
        queue.enqueue(1);
        queue.enqueue(2);

        for _ in 0..100 {
            queue.enqueue_front(2);
        }

        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn contains_tracks_queue_membership() {
        let mut queue = DeduplicatedQueue::default();

        assert!(!queue.contains(1));
        queue.enqueue(1);
        assert_eq!(queue.len(), 1);
        assert!(queue.contains(1));
        assert_eq!(queue.pop(), Some(1));
        assert!(!queue.contains(1));
    }

    #[test]
    fn vector_conversion_preserves_order_and_deduplicates() {
        let mut queue = DeduplicatedQueue::from(vec![2, 1, 2, 3]);

        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(3));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn remove_drops_pending_value() {
        let mut queue = DeduplicatedQueue::default();
        queue.enqueue(1);
        queue.enqueue(2);

        assert!(queue.remove(1));
        assert!(!queue.remove(3));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn pop_min_by_key_preserves_fifo_within_best_rank() {
        let mut queue = DeduplicatedQueue::from(vec![4, 2, 3, 1]);

        assert_eq!(queue.pop_min_by_key(|value| value % 2), Some(4));
        assert_eq!(queue.pop_min_by_key(|value| value % 2), Some(2));
        assert_eq!(queue.pop_min_by_key(|value| value % 2), Some(3));
        assert_eq!(queue.pop_min_by_key(|value| value % 2), Some(1));
        assert_eq!(queue.pop_min_by_key(|value| value % 2), None);
    }

    #[test]
    fn pop_min_by_key_ignores_stale_priority_promotions() {
        let mut queue = DeduplicatedQueue::default();
        queue.enqueue(3);
        queue.enqueue(1);
        queue.enqueue_front(1);

        assert_eq!(queue.pop_min_by_key(|value| value), Some(1));
        assert_eq!(queue.pop_min_by_key(|value| value), Some(3));
        assert_eq!(queue.pop_min_by_key(|value| value), None);
    }

    #[test]
    fn pop_min_where_by_key_preserves_filtered_entries() {
        let mut queue = DeduplicatedQueue::from(vec![4, 3, 2, 1]);

        assert_eq!(
            queue.pop_min_where_by_key(|value| value % 2 == 1, |value| value),
            Some(1)
        );
        assert!(queue.contains(2));
        assert!(queue.contains(3));
        assert!(queue.contains(4));

        assert_eq!(
            queue.pop_min_where_by_key(|value| value > 2, |value| value),
            Some(3)
        );
        assert_eq!(queue.pop(), Some(4));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn pop_where_preserves_deferred_entries() {
        let mut queue = DeduplicatedQueue::default();
        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue(3);

        assert_eq!(queue.pop_where(|value| value % 2 == 0), Some(2));
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(3));
    }

    #[test]
    fn revision_changes_only_with_logical_queue_mutation() {
        let mut queue = DeduplicatedQueue::default();
        let initial = queue.revision();

        assert!(queue.enqueue(1));
        let after_enqueue = queue.revision();
        assert_ne!(after_enqueue, initial);
        assert!(!queue.enqueue(1));
        assert_eq!(queue.revision(), after_enqueue);

        queue.enqueue_front(1);
        let after_promotion = queue.revision();
        assert_ne!(after_promotion, after_enqueue);
        assert!(!queue.remove(2));
        assert_eq!(queue.revision(), after_promotion);

        assert_eq!(queue.pop(), Some(1));
        assert_ne!(queue.revision(), after_promotion);
    }

    #[test]
    fn clear_reuses_queue_for_new_values() {
        let mut queue = DeduplicatedQueue::from(vec![1, 2]);
        let before_clear = queue.revision();

        queue.clear();

        assert_eq!(queue.len(), 0);
        assert_ne!(queue.revision(), before_clear);
        queue.enqueue(3);
        assert_eq!(queue.pop(), Some(3));
        assert_eq!(queue.pop(), None);
    }
}
