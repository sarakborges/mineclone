use std::sync::Arc;

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, fluid::FluidRegistry,
        layer::LayerRegistry, secondary_property::SecondaryPropertyRegistry,
    },
    voxel::mesh_snapshot::{ChunkMeshDependencies, ChunkMeshSnapshot},
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::{BuiltChunkMesh, ChunkMeshBuildContext, build_chunk_render_meshes},
    chunk_system_params::ChunkContent,
    chunk_task_queue::{ChunkTaskQueue, CompletedChunkTask},
};

pub(crate) const MAX_MESH_TASKS_IN_FLIGHT: usize = 8;

pub(crate) struct MeshContentSnapshot {
    blocks: BlockRegistry,
    layers: LayerRegistry,
    fluids: FluidRegistry,
    biomes: BiomeRegistry,
    secondary_properties: SecondaryPropertyRegistry,
    biome_field: BiomeField,
}

impl MeshContentSnapshot {
    pub(crate) fn from_content(content: &ChunkContent<'_>) -> Self {
        Self {
            blocks: content.blocks().clone(),
            layers: content.layers().clone(),
            fluids: content.fluids().clone(),
            biomes: BiomeRegistry::clone(&content.biomes),
            secondary_properties: content.secondary_properties().clone(),
            biome_field: content.biome_field.as_ref().clone(),
        }
    }

    pub(crate) fn context<'a>(
        &'a self,
        world: &'a ChunkMeshSnapshot,
    ) -> ChunkMeshBuildContext<'a, ChunkMeshSnapshot> {
        ChunkMeshBuildContext {
            world,
            blocks: &self.blocks,
            layers: &self.layers,
            fluids: &self.fluids,
            biomes: &self.biomes,
            secondary_properties: &self.secondary_properties,
            biome_field: &self.biome_field,
        }
    }
}

pub(crate) struct ChunkMeshTaskOutput {
    pub(crate) meshes: Vec<BuiltChunkMesh>,
    pub(crate) dependencies: ChunkMeshDependencies,
}

#[derive(Resource, Default)]
pub(crate) struct ChunkMeshTasks {
    revision: u64,
    snapshot: Option<Arc<MeshContentSnapshot>>,
    pending: ChunkTaskQueue<ChunkMeshTaskOutput>,
}

impl ChunkMeshTasks {
    pub(crate) fn sync_snapshot(&mut self, content: &ChunkContent<'_>) {
        if self.snapshot.is_some() && !content.mesh_inputs_changed() {
            return;
        }

        self.revision = self.revision.wrapping_add(1).max(1);
        self.snapshot = Some(Arc::new(MeshContentSnapshot::from_content(content)));
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn contains(&self, coord: IVec3) -> bool {
        self.pending.contains(coord)
    }

    pub(crate) fn schedule(&mut self, coord: IVec3, world: ChunkMeshSnapshot) -> bool {
        if self.pending.len() >= MAX_MESH_TASKS_IN_FLIGHT || self.pending.contains(coord) {
            return false;
        }

        let snapshot = self
            .snapshot
            .as_ref()
            .unwrap_or_else(|| panic!("chunk mesh snapshot must be prepared before scheduling"))
            .clone();
        let revision = self.revision;
        let dependencies = world.dependencies();
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let world = world.materialize_shell();
            let context = snapshot.context(&world);
            ChunkMeshTaskOutput {
                meshes: build_chunk_render_meshes(coord, world.chunk(), &context),
                dependencies,
            }
        });

        self.pending.insert(coord, revision, task)
    }

    pub(crate) fn cancel_farthest_where(
        &mut self,
        center: IVec3,
        predicate: impl FnMut(IVec3) -> bool,
    ) -> Option<IVec3> {
        self.pending.cancel_farthest_where(center, predicate)
    }

    pub(crate) fn poll_ready(&mut self) -> Option<CompletedChunkTask<ChunkMeshTaskOutput>> {
        self.pending.poll_ready()
    }
}
