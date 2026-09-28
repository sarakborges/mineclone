use bevy::{platform::collections::HashMap, prelude::IVec3};

#[derive(Clone, Default)]
pub(super) struct ObjectRevisionState {
    chunk_revisions: HashMap<IVec3, u64>,
    next_chunk_revision: u64,
    scene_revision: u64,
}

impl ObjectRevisionState {
    pub(super) fn scene_revision(&self) -> u64 {
        self.scene_revision
    }

    pub(super) fn chunk_revision(&self, coord: IVec3) -> Option<u64> {
        self.chunk_revisions.get(&coord).copied()
    }

    pub(super) fn mark_chunk_changed(&mut self, coord: IVec3) {
        self.bump_scene_revision();
        self.next_chunk_revision = self
            .next_chunk_revision
            .checked_add(1)
            .expect("chunk object revision counter exhausted");
        self.chunk_revisions.insert(coord, self.next_chunk_revision);
    }

    pub(super) fn remove_chunk(&mut self, coord: IVec3) -> Option<u64> {
        let revision = self.chunk_revisions.remove(&coord)?;
        self.bump_scene_revision();
        Some(revision)
    }

    fn bump_scene_revision(&mut self) {
        self.scene_revision = self
            .scene_revision
            .checked_add(1)
            .expect("world object scene revision counter exhausted");
    }
}
