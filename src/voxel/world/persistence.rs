use std::{io, sync::Arc};

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::IVec3,
};

use crate::content::{
    block::BlockRegistry, fluid::FluidRegistry, layer::LayerRegistry, object::ObjectRegistry,
};
use crate::voxel::{chunk::VoxelChunk, chunk_archive::ArchivedChunk, chunk_disk::DiskChunk};

use super::VoxelWorld;

#[derive(Clone, Default)]
pub(super) struct ChunkPersistenceState {
    archived_chunks: HashMap<IVec3, Arc<ArchivedChunk>>,
    persistent_chunks: HashSet<IVec3>,
}

impl ChunkPersistenceState {
    pub(super) fn mark_persistent(&mut self, coord: IVec3) {
        self.persistent_chunks.insert(coord);
    }

    pub(super) fn is_persistent(&self, coord: IVec3) -> bool {
        self.persistent_chunks.contains(&coord)
    }

    pub(super) fn has_archived(&self, coord: IVec3) -> bool {
        self.archived_chunks.contains_key(&coord)
    }

    /// Any chunk that reached runtime residency has crossed the persistence
    /// boundary. Archiving must therefore retain it even when the player never
    /// mutated it and even when its voxel payload is empty.
    pub(super) fn archive_if_persistent(&mut self, coord: IVec3, chunk: &VoxelChunk) {
        self.mark_persistent(coord);
        self.archived_chunks
            .insert(coord, Arc::new(ArchivedChunk::from_chunk(chunk)));
    }

    pub(super) fn restore(&mut self, coord: IVec3) -> Option<VoxelChunk> {
        self.archived_chunks
            .remove(&coord)
            .map(|archived| archived.restore())
    }

    pub(super) fn persistent_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.persistent_chunks.iter().copied()
    }

    pub(super) fn archived_chunk(&self, coord: IVec3) -> Option<&ArchivedChunk> {
        self.archived_chunks.get(&coord).map(Arc::as_ref)
    }

    pub(super) fn insert_saved(&mut self, coord: IVec3, archived: ArchivedChunk) {
        self.persistent_chunks.insert(coord);
        self.archived_chunks.insert(coord, Arc::new(archived));
    }
}

impl VoxelWorld {
    /// Every chunk that has entered the playable/materialized world is part of
    /// persistent spatial state. Resident chunks are included directly; chunks
    /// that were unloaded are retained by `ChunkPersistenceState`.
    pub(crate) fn persistent_chunk_coords(&self) -> impl Iterator<Item = IVec3> {
        let mut coords = self
            .persistence
            .persistent_coords()
            .collect::<HashSet<_>>();
        coords.extend(self.resident.coords());
        coords.into_iter()
    }

    pub(crate) fn save_persistent_chunk(
        &self,
        coord: IVec3,
        fluids: &FluidRegistry,
    ) -> io::Result<DiskChunk> {
        let resident = self.resident.get(coord);
        if resident.is_none() && !self.persistence.is_persistent(coord) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("chunk {coord:?} was never materialized"),
            ));
        }

        if let Some(chunk) = resident {
            DiskChunk::from_chunk(coord, chunk, fluids)
        } else if let Some(archived) = self.persistence.archived_chunk(coord) {
            DiskChunk::from_archived_chunk(coord, archived, fluids)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("materialized chunk {coord:?} has neither resident nor archived content"),
            ))
        }
    }

    pub(crate) fn insert_saved_chunk(
        &mut self,
        entry: DiskChunk,
        blocks: &BlockRegistry,
        layers: &LayerRegistry,
        objects: &ObjectRegistry,
        fluids: &FluidRegistry,
    ) -> io::Result<()> {
        let coord = entry.coord()?;
        if self.persistence.is_persistent(coord) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("duplicate saved chunk coordinate: {coord:?}"),
            ));
        }

        let (coord, archived) = entry.into_archived_chunk(blocks, layers, objects, fluids)?;
        self.persistence.insert_saved(coord, archived);
        Ok(())
    }
}
