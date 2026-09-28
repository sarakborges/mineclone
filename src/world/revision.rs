/// Identifies the immutable input snapshot used by an asynchronous world job.
///
/// This is intentionally distinct from content, residency, lighting and
/// presentation revisions. A task result is publishable only when this value
/// still matches the input snapshot owner that scheduled it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TaskInputRevision(u64);

impl From<u64> for TaskInputRevision {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl PartialEq<u64> for TaskInputRevision {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_input_revision_only_matches_the_snapshot_that_created_it() {
        let revision = TaskInputRevision::from(7);

        assert_eq!(revision, 7);
        assert_ne!(revision, 8);
    }
}
