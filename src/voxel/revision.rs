#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChunkContentRevision(u64);

impl ChunkContentRevision {
    pub(crate) fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}
