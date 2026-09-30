/// Identifies the immutable input snapshot used by an asynchronous world job.
///
/// This is intentionally distinct from content, residency, lighting and
/// presentation revisions. A task result is publishable only when this value
/// still matches the input snapshot owner that scheduled it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct TaskInputRevision(u64);

impl TaskInputRevision {
    pub(crate) fn next(self) -> Self {
        Self(self.0.wrapping_add(1).max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_input_revision_starts_uninitialized_and_advances_without_zero() {
        let initial = TaskInputRevision::default();
        let first = initial.next();
        let second = first.next();

        assert_ne!(initial, first);
        assert_ne!(first, second);
        assert_eq!(first, TaskInputRevision(1));
        assert_eq!(second, TaskInputRevision(2));
    }

    #[test]
    fn task_input_revision_wraps_to_first_live_revision() {
        assert_eq!(TaskInputRevision(u64::MAX).next(), TaskInputRevision(1));
    }
}
