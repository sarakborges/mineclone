use bevy::{platform::collections::HashMap, prelude::IVec3};

#[derive(Clone, Copy, Default)]
struct ChunkObjectRevision(u64);

impl ChunkObjectRevision {
    fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }

    fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Default)]
struct ObjectSceneRevision(u64);

impl ObjectSceneRevision {
    fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }

    fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Default)]
pub(super) struct ObjectRevisionState {
    chunk_revisions: HashMap<IVec3, ChunkObjectRevision>,
    next_chunk_revision: ChunkObjectRevision,
    scene_revision: ObjectSceneRevision,
}

impl ObjectRevisionState {
    pub(super) fn scene_revision(&self) -> u64 {
        self.scene_revision.raw()
    }

    pub(super) fn chunk_revision(&self, coord: IVec3) -> Option<u64> {
        self.chunk_revisions.get(&coord).copied().map(ChunkObjectRevision::raw)
    }

    pub(super) fn mark_chunk_changed(&mut self, coord: IVec3) {
        self.bump_scene_revision();
        self.next_chunk_revision = self
            .next_chunk_revision
            .checked_next()
            .expect("chunk object revision counter exhausted");
        self.chunk_revisions.insert(coord, self.next_chunk_revision);
    }

    pub(super) fn remove_chunk(&mut self, coord: IVec3) -> Option<u64> {
        let revision = self.chunk_revisions.remove(&coord)?;
        self.bump_scene_revision();
        Some(revision.raw())
    }

    fn bump_scene_revision(&mut self) {
        self.scene_revision = self
            .scene_revision
            .checked_next()
            .expect("world object scene revision counter exhausted");
    }
}
