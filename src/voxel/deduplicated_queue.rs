use std::{
    collections::{HashSet, VecDeque},
    hash::Hash,
};

#[derive(Debug)]
pub(crate) struct DeduplicatedQueue<T> {
    pending: VecDeque<T>,
    queued: HashSet<T>,
}

impl<T> Default for DeduplicatedQueue<T> {
    fn default() -> Self {
        Self {
            pending: VecDeque::new(),
            queued: HashSet::new(),
        }
    }
}

impl<T> From<Vec<T>> for DeduplicatedQueue<T>
where
    T: Copy + Eq + Hash,
{
    fn from(values: Vec<T>) -> Self {
        let mut queue = Self::default();
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
    pub(crate) fn enqueue(&mut self, value: T) -> bool {
        if !self.queued.insert(value) {
            return false;
        }

        self.pending.push_back(value);
        true
    }

    pub(crate) fn enqueue_front(&mut self, value: T) {
        if !self.queued.insert(value) {
            self.pending.retain(|pending| *pending != value);
        }

        self.pending.push_front(value);
    }

    pub(crate) fn contains(&self, value: T) -> bool {
        self.queued.contains(&value)
    }

    pub(crate) fn remove(&mut self, value: T) -> bool {
        if !self.queued.remove(&value) {
            return false;
        }

        self.pending.retain(|pending| *pending != value);
        true
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        let value = self.pending.pop_front()?;
        self.queued.remove(&value);
        Some(value)
    }

    pub(crate) fn pop_where(&mut self, mut predicate: impl FnMut(T) -> bool) -> Option<T> {
        let index = self.pending.iter().position(|value| predicate(*value))?;
        let value = self
            .pending
            .remove(index)
            .expect("located queue index must remain valid");
        self.queued.remove(&value);
        Some(value)
    }

    pub(crate) fn len(&self) -> usize {
        self.pending.len()
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
    fn contains_tracks_queue_membership() {
        let mut queue = DeduplicatedQueue::default();

        assert!(!queue.contains(1));
        queue.enqueue(1);
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
    fn pop_where_preserves_deferred_entries() {
        let mut queue = DeduplicatedQueue::default();
        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue(3);

        assert_eq!(queue.pop_where(|value| value % 2 == 0), Some(2));
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(3));
    }
}
