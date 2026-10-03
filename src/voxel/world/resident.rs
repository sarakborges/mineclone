use bevy::{platform::collections::HashMap, prelude::*};

use super::{super::chunk::VoxelChunk, resident_index::LoadedChunkColumnIndex};

#[derive(Clone, Default)]
pub(super) struct ResidentChunkStore {
    chunks: HashMap<IVec3, VoxelChunk>,
    columns: LoadedChunkColumnIndex,
}

impl ResidentChunkStore {
    pub(super) fn insert(&mut self, coord: IVec3, chunk: VoxelChunk) {
        let previous = self.chunks.insert(coord, chunk);
        assert!(
            previous.is_none(),
            "resident chunk already existed at {coord:?}"
        );
        self.columns.insert(coord);
    }

    pub(super) fn remove(&mut self, coord: IVec3) -> Option<VoxelChunk> {
        let chunk = self.chunks.remove(&coord)?;
        self.columns.remove(coord);
        Some(chunk)
    }

    pub(super) fn get(&self, coord: IVec3) -> Option<&VoxelChunk> {
        self.chunks.get(&coord)
    }

    pub(super) fn get_mut(&mut self, coord: IVec3) -> Option<&mut VoxelChunk> {
        self.chunks.get_mut(&coord)
    }

    pub(super) fn contains(&self, coord: IVec3) -> bool {
        self.chunks.contains_key(&coord)
    }

    pub(super) fn coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.chunks.keys().copied()
    }

    pub(super) fn coords_below(&self, coord: IVec3) -> impl Iterator<Item = IVec3> + '_ {
        self.columns.coords_below(coord)
    }

    pub(super) fn highest_world_y_in_column(&self, horizontal_chunk: IVec2) -> Option<i32> {
        self.columns.highest_world_y_in_column(horizontal_chunk)
    }
}
