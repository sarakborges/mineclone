use std::io;

use serde::{Deserialize, Serialize};

use crate::{
    content::{block::BlockRegistry, fluid::FluidRegistry},
    voxel::{chunk_disk::DiskChunk, world::VoxelWorld},
};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub(super) struct SavedChunkCatalog(Vec<DiskChunk>);

impl SavedChunkCatalog {
    pub(super) fn capture(world: &VoxelWorld, fluids: &FluidRegistry) -> io::Result<Self> {
        Ok(Self(world.save_persistent_chunks(fluids)?))
    }

    pub(super) fn from_disk_chunks(chunks: Vec<DiskChunk>) -> Self {
        Self(chunks)
    }

    pub(super) fn disk_chunks(&self) -> &[DiskChunk] {
        &self.0
    }

    pub(super) fn into_world(
        self,
        blocks: &BlockRegistry,
        fluids: &FluidRegistry,
    ) -> io::Result<VoxelWorld> {
        VoxelWorld::from_saved_chunks(self.0, blocks, fluids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_chunk_catalog_keeps_legacy_array_shape() {
        let catalog = SavedChunkCatalog::default();
        assert_eq!(serde_json::to_string(&catalog).unwrap(), "[]");
    }
}
