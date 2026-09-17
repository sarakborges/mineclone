use std::{collections::HashSet, io};

use crate::content::{block::BlockRegistry, fluid::FluidRegistry};
use crate::voxel::{chunk_archive::ArchivedChunk, chunk_disk::DiskChunk};

use super::VoxelWorld;

impl VoxelWorld {
    /// Captures only modified chunks, including ones unloaded from RAM-facing
    /// resident storage. Generated but unmodified terrain is seed-derived.
    pub(crate) fn save_modified_chunks(&self, fluids: &FluidRegistry) -> io::Result<Vec<DiskChunk>> {
        let mut coords = self.dirty_chunks.iter().copied().collect::<Vec<_>>();
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
                        format!("modified chunk {coord:?} has neither loaded nor archived content"),
                    ))
                }
            })
            .collect()
    }

    /// Rebuild a *fresh* world from disk. Keep all saved chunks archived until
    /// streaming needs them; don't rehydrate the entire explored map at once.
    /// Validate every chunk and duplicate coordinate before any world is exposed.
    pub(crate) fn from_saved_chunks(
        saved: Vec<DiskChunk>,
        blocks: &BlockRegistry,
        fluids: &FluidRegistry,
    ) -> io::Result<Self> {
        let mut decoded = Vec::with_capacity(saved.len());
        let mut seen = HashSet::with_capacity(saved.len());
        for entry in saved {
            let (coord, chunk) = entry.into_chunk(blocks, fluids)?;
            if !seen.insert(coord) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("duplicate saved chunk coordinate: {coord:?}"),
                ));
            }
            decoded.push((coord, ArchivedChunk::from_chunk(&chunk)));
        }

        let mut world = Self::default();
        for (coord, archived) in decoded {
            world.generated_chunks.insert(coord);
            world.dirty_chunks.insert(coord);
            world.archived_chunks.insert(coord, archived);
        }
        Ok(world)
    }
}
