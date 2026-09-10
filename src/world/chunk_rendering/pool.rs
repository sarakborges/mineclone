use std::collections::HashMap;

use bevy::prelude::*;

struct ChunkRenderSlot {
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
    mesh_bytes: usize,
}

#[derive(Resource, Default)]
pub struct ChunkRenderPool {
    active: HashMap<IVec3, ChunkRenderSlot>,
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

    pub(super) fn insert(
        &mut self,
        coord: IVec3,
        entities: Vec<Entity>,
        meshes: Vec<Handle<Mesh>>,
        mesh_bytes: usize,
    ) {
        self.active.insert(
            coord,
            ChunkRenderSlot {
                entities,
                meshes,
                mesh_bytes,
            },
        );
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
