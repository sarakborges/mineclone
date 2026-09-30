use crate::voxel::revision::BlockTopologyRevision;

#[derive(Clone, Default)]
pub(super) struct BlockRevisionState {
    topology: BlockTopologyRevision,
}

impl BlockRevisionState {
    pub(super) fn topology(&self) -> BlockTopologyRevision {
        self.topology
    }

    pub(super) fn mark_topology_changed(&mut self) {
        self.topology = self
            .topology
            .checked_next()
            .expect("block topology revision counter exhausted");
    }
}
