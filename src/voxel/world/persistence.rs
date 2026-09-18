use std::{io, sync::Arc};

use bevy::prelude::IVec3;

use crate::content::{block::BlockRegistry, fluid::FluidRegistry};
use crate::voxel::{chunk_archive::ArchivedChunk, chunk_disk::DiskChunk};

use super::VoxelWorld;

impl VoxelWorld {
    /// Monotonic revision of authoritative persistent world content. Generating
    /// a chunk for the first time and block/fluid mutations advance it;
    /// archiving, restoring and lighting do not.
    pub(crate) fn save_content_revision(&self) -> u64 {
        self.save_revision
    }

    /// Captures every chunk that has ever been generated in this world,
    /// including chunks currently archived out of resident storage. Generated
    /// terrain is authoritative state after first creation and must never be
    /// reconstructed from the seed once it exists.
    pub(crate) fn save_generated_chunks(&self, fluids: &FluidRegistry) -> io::Result<Vec<DiskChunk>> {
        let mut coords = self.generated_chunks.iter().copied().collect::<Vec<_>>();
        coords.sort_unstable_by_key(|coord| (coord.x, coord.y, coord.z));
        coords
            .into_iter()
            .map(|coord| {
                if let Some(chunk) = self.chunks.get(&coord) {
                    DiskChunk::from_chunk(coord, chunk, fluids)
                } else if let Some(archived) = self.archived_chunks.get(&coord) {
                    DiskChunk::from_chunk(coord, &archived.restore(), fluids)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("generated chunk {coord:?} has neither loaded nor archived content"),
                    ))
                }
            })
            .collect()
    }

    /// Rebuild a *fresh* world from disk, archiving saved chunks until streaming
    /// needs them. Reject duplicate coordinates BEFORE decoding their content.
    /// On any error the partially built world is dropped without being exposed.
    pub(crate) fn from_saved_chunks(
        saved: Vec<DiskChunk>,
        blocks: &BlockRegistry,
        fluids: &FluidRegistry,
    ) -> io::Result<Self> {
        let mut world = Self::default();
        for entry in saved {
            let coord = IVec3::from_array(entry.coord);
            if world.generated_chunks.contains(&coord) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("duplicate saved chunk coordinate: {coord:?}"),
                ));
            }
            let (coord, chunk) = entry.into_chunk(blocks, fluids)?;
            let archived = Arc::new(ArchivedChunk::from_chunk(&chunk));
            world.generated_chunks.insert(coord);
            world.archived_chunks.insert(coord, archived);
        }
        Ok(world)
    }
}
