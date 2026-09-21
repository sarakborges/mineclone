use std::{io, sync::Arc};

use bevy::prelude::IVec3;

use crate::content::{block::BlockRegistry, fluid::FluidRegistry, layer::LayerRegistry};
use crate::voxel::chunk_disk::DiskChunk;

use super::VoxelWorld;

impl VoxelWorld {
    /// Captures only chunks with persistent mutations. Untouched deterministic
    /// terrain is reconstructed from the seed after load instead of being kept
    /// in RAM and copied into every save.
    pub(crate) fn persistent_chunk_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.persistent_chunks.iter().copied()
    }

    pub(crate) fn save_persistent_chunk(
        &self,
        coord: IVec3,
        fluids: &FluidRegistry,
    ) -> io::Result<DiskChunk> {
        if !self.persistent_chunks.contains(&coord) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("chunk {coord:?} is not persistent"),
            ));
        }

        if let Some(chunk) = self.chunks.get(&coord) {
            DiskChunk::from_chunk(coord, chunk, fluids)
        } else if let Some(archived) = self.archived_chunks.get(&coord) {
            DiskChunk::from_archived_chunk(coord, archived, fluids)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("persistent chunk {coord:?} has neither loaded nor archived content"),
            ))
        }
    }

    /// Rebuild a *fresh* world from disk, archiving saved chunks until streaming
    /// needs them. Reject duplicate coordinates BEFORE decoding their content.
    /// On any error the partially built world is dropped without being exposed.
    pub(crate) fn from_saved_chunks(
        saved: Vec<DiskChunk>,
        blocks: &BlockRegistry,
        layers: &LayerRegistry,
        fluids: &FluidRegistry,
    ) -> io::Result<Self> {
        let mut world = Self::default();
        for entry in saved {
            let coord = entry.coord()?;
            if world.persistent_chunks.contains(&coord) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("duplicate saved chunk coordinate: {coord:?}"),
                ));
            }
            let (coord, archived) = entry.into_archived_chunk(blocks, layers, fluids)?;
            world.persistent_chunks.insert(coord);
            world.archived_chunks.insert(coord, Arc::new(archived));
        }
        Ok(world)
    }
}
