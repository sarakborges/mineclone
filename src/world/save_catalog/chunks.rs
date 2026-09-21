use std::io;

use crate::{
    content::{block::BlockRegistry, fluid::FluidRegistry, layer::LayerRegistry},
    voxel::{chunk_disk::DiskChunk, world::VoxelWorld},
};

#[derive(Clone, Debug, Default)]
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
        layers: &LayerRegistry,
        fluids: &FluidRegistry,
    ) -> io::Result<VoxelWorld> {
        VoxelWorld::from_saved_chunks(self.0, blocks, layers, fluids)
    }
}

