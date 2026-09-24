use std::io;

use crate::{
    content::{
        block::BlockRegistry, fluid::FluidRegistry, layer::LayerRegistry,
        object::ObjectRegistry,
    },
    voxel::{chunk_disk::DiskChunk, world::VoxelWorld},
};

#[derive(Clone, Debug, Default)]
pub(super) struct SavedChunkCatalog(Vec<DiskChunk>);

impl SavedChunkCatalog {
    pub(super) fn from_disk_chunks(chunks: Vec<DiskChunk>) -> Self {
        Self(chunks)
    }

    pub(super) fn into_world(
        self,
        blocks: &BlockRegistry,
        layers: &LayerRegistry,
        objects: &ObjectRegistry,
        fluids: &FluidRegistry,
    ) -> io::Result<VoxelWorld> {
        VoxelWorld::from_saved_chunks(self.0, blocks, layers, objects, fluids)
    }
}

