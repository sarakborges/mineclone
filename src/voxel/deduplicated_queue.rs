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

    pub(crate) fn pop(&mut self) -> Option<T> {
        let value = self.pending.pop_front()?;
        self.queued.remove(&value);
        Some(value)
    }

    pub(crate) fn len(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn clear(&mut self) {
        self.pending.clear();
        self.queued.clear();
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
}
