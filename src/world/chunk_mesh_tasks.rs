use std::{collections::HashMap, sync::Arc};

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, fluid::FluidRegistry,
        secondary_property::SecondaryPropertyRegistry,
    },
    voxel::mesh_snapshot::ChunkMeshSnapshot,
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::{BuiltChunkMesh, ChunkMeshBuildContext, build_chunk_render_meshes},
    chunk_system_params::ChunkContent,
};

pub(crate) const MAX_MESH_TASKS_IN_FLIGHT: usize = 8;

struct MeshContentSnapshot {
    blocks: BlockRegistry,
    fluids: FluidRegistry,
    biomes: BiomeRegistry,
    secondary_properties: SecondaryPropertyRegistry,
    biome_field: BiomeField,
}

impl MeshContentSnapshot {
    fn from_content(content: &ChunkContent<'_>) -> Self {
        Self {
            blocks: content.blocks().clone(),
            fluids: content.fluids().clone(),
            biomes: BiomeRegistry::clone(&content.biomes),
            secondary_properties: content.secondary_properties().clone(),
            biome_field: content.biome_field.as_ref().clone(),
        }
    }

    fn context<'a>(
        &'a self,
        world: &'a ChunkMeshSnapshot,
    ) -> ChunkMeshBuildContext<'a, ChunkMeshSnapshot> {
        ChunkMeshBuildContext {
            world,
            blocks: &self.blocks,
            fluids: &self.fluids,
            biomes: &self.biomes,
            secondary_properties: &self.secondary_properties,
            biome_field: &self.biome_field,
        }
    }
}

struct PendingMesh {
    revision: u64,
    task: Task<Vec<BuiltChunkMesh>>,
}

pub(crate) struct CompletedChunkMesh {
    pub(crate) coord: IVec3,
    pub(crate) revision: u64,
    pub(crate) meshes: Vec<BuiltChunkMesh>,
}

#[derive(Resource, Default)]
pub(crate) struct ChunkMeshTasks {
    revision: u64,
    snapshot: Option<Arc<MeshContentSnapshot>>,
    pending: HashMap<IVec3, PendingMesh>,
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
        self.pending.contains_key(&coord)
    }

    pub(crate) fn schedule(&mut self, coord: IVec3, world: ChunkMeshSnapshot) -> bool {
        if self.pending.len() >= MAX_MESH_TASKS_IN_FLIGHT || self.pending.contains_key(&coord) {
            return false;
        }

        let snapshot = self
            .snapshot
            .as_ref()
            .unwrap_or_else(|| panic!("chunk mesh snapshot must be prepared before scheduling"))
            .clone();
        let revision = self.revision;
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let context = snapshot.context(&world);
            build_chunk_render_meshes(coord, world.chunk(), &context)
        });

        self.pending.insert(coord, PendingMesh { revision, task });
        true
    }

    pub(crate) fn collect_ready(&mut self, maximum: usize) -> Vec<CompletedChunkMesh> {
        if maximum == 0 || self.pending.is_empty() {
            return Vec::new();
        }

        let coords = self.pending.keys().copied().collect::<Vec<_>>();
        let mut completed = Vec::new();

        for coord in coords {
            if completed.len() >= maximum {
                break;
            }

            let ready = {
                let pending = self
                    .pending
                    .get_mut(&coord)
                    .unwrap_or_else(|| panic!("pending mesh disappeared for {coord:?}"));
                check_ready(&mut pending.task).map(|meshes| (pending.revision, meshes))
            };

            let Some((revision, meshes)) = ready else {
                continue;
            };
            self.pending.remove(&coord);
            completed.push(CompletedChunkMesh {
                coord,
                revision,
                meshes,
            });
        }

        completed
    }
}
