use std::collections::HashMap;

use bevy::prelude::*;

use crate::{content::fluid::FluidId, voxel::block_face::BlockFace};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChunkMeshKey {
    Terrain {
        block_id: &'static str,
        face: BlockFace,
        casts_shadow: bool,
    },
    Fluid(FluidId),
}

#[derive(Default)]
pub(super) struct ChunkRenderAllocation {
    pub(super) entities: Vec<Entity>,
    pub(super) meshes: Vec<Handle<Mesh>>,
    pub(super) mesh_keys: Vec<ChunkMeshKey>,
    pub(super) fluid_ids: Vec<FluidId>,
    pub(super) mesh_bytes: usize,
    pub(super) fluid_mesh_bytes: usize,
}

#[derive(Resource, Default)]
pub struct ChunkRenderPool {
    active: HashMap<IVec3, ChunkRenderAllocation>,
}

impl ChunkRenderPool {
    pub fn contains(&self, coord: IVec3) -> bool {
        self.active.contains_key(&coord)
    }

    pub fn active_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.active.keys().copied()
    }

    pub(crate) fn active_count(&self) -> usize {
        self.active.len()
    }

    pub(crate) fn mesh_count(&self) -> usize {
        self.active.values().map(|slot| slot.meshes.len()).sum()
    }

    pub(crate) fn mesh_bytes(&self) -> usize {
        self.active.values().map(|slot| slot.mesh_bytes).sum()
    }

    pub fn take(&mut self, coord: IVec3) -> Option<(Vec<Entity>, Vec<Handle<Mesh>>)> {
        self.active
            .remove(&coord)
            .map(|slot| (slot.entities, slot.meshes))
    }

    pub(super) fn replace_mesh_assets(
        &mut self,
        coord: IVec3,
        meshes: &mut Assets<Mesh>,
        replacement_keys: &[ChunkMeshKey],
        replacements: Vec<Mesh>,
        mesh_bytes: usize,
    ) -> bool {
        let Some(slot) = self.active.get_mut(&coord) else {
            return false;
        };
        if slot.mesh_keys.as_slice() != replacement_keys
            || slot.meshes.len() != replacements.len()
            || slot.meshes.iter().any(|handle| !meshes.contains(handle))
        {
            return false;
        }

        for (handle, replacement) in slot.meshes.iter().zip(replacements) {
            let Some(mut existing) = meshes.get_mut(handle) else {
                return false;
            };
            *existing = replacement;
        }
        slot.mesh_bytes = mesh_bytes;
        true
    }

    pub(super) fn replace_fluid_mesh_assets(
        &mut self,
        coord: IVec3,
        meshes: &mut Assets<Mesh>,
        replacements: Vec<(FluidId, Mesh)>,
        fluid_mesh_bytes: usize,
    ) -> bool {
        let Some(slot) = self.active.get_mut(&coord) else {
            return false;
        };
        if slot.fluid_ids.len() != replacements.len()
            || slot
                .fluid_ids
                .iter()
                .zip(&replacements)
                .any(|(existing, (replacement, _))| existing != replacement)
        {
            return false;
        }

        let terrain_mesh_count = slot.meshes.len().saturating_sub(slot.fluid_ids.len());
        let fluid_handles = &slot.meshes[terrain_mesh_count..];
        if fluid_handles.iter().any(|handle| !meshes.contains(handle)) {
            return false;
        }

        for (handle, (_, replacement)) in fluid_handles.iter().zip(replacements) {
            let Some(mut existing) = meshes.get_mut(handle) else {
                return false;
            };
            *existing = replacement;
        }

        slot.mesh_bytes = slot
            .mesh_bytes
            .saturating_sub(slot.fluid_mesh_bytes)
            .saturating_add(fluid_mesh_bytes);
        slot.fluid_mesh_bytes = fluid_mesh_bytes;
        true
    }

    pub(super) fn insert(&mut self, coord: IVec3, allocation: ChunkRenderAllocation) {
        self.active.insert(coord, allocation);
    }

    fn clear(&mut self, meshes: &mut Assets<Mesh>) {
        for (_, slot) in self.active.drain() {
            for handle in slot.meshes {
                let _ = meshes.remove(&handle);
            }
        }
    }
}

pub fn clear_chunk_render_pool(
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_pool: ResMut<ChunkRenderPool>,
) {
    render_pool.clear(&mut meshes);
}
