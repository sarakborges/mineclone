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

pub(super) struct DetachedRenderAllocationParts {
    pub(super) entities: Vec<Entity>,
    pub(super) meshes: Vec<Handle<Mesh>>,
}

#[derive(Resource, Default)]
pub struct ChunkRenderPool {
    active: HashMap<IVec3, ChunkRenderAllocation>,
    membership_revision: u64,
}

impl ChunkRenderPool {
    pub(crate) fn contains(&self, coord: IVec3) -> bool {
        self.active.contains_key(&coord)
    }

    pub(crate) fn membership_revision(&self) -> u64 {
        self.membership_revision
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

    fn take(&mut self, coord: IVec3) -> Option<(Vec<Entity>, Vec<Handle<Mesh>>)> {
        let slot = self.active.remove(&coord)?;
        self.bump_membership_revision();
        Some((slot.entities, slot.meshes))
    }

    pub(super) fn replace_terrain_mesh_assets(
        &mut self,
        coord: IVec3,
        meshes: &mut Assets<Mesh>,
        replacement_keys: &[ChunkMeshKey],
        replacements: &mut Vec<Mesh>,
        terrain_mesh_bytes: usize,
    ) -> bool {
        let Some(slot) = self.active.get_mut(&coord) else {
            return false;
        };
        let Some(terrain_mesh_count) = fluid_mesh_start(slot) else {
            return false;
        };
        if &slot.mesh_keys[..terrain_mesh_count] != replacement_keys
            || terrain_mesh_count != replacements.len()
            || slot.meshes[..terrain_mesh_count]
                .iter()
                .any(|handle| !meshes.contains(handle))
        {
            return false;
        }

        for (handle, replacement) in slot.meshes[..terrain_mesh_count]
            .iter()
            .zip(replacements.drain(..))
        {
            let mut existing = meshes
                .get_mut(handle)
                .expect("terrain mesh handle was checked before replacement");
            *existing = replacement;
        }
        slot.mesh_bytes = terrain_mesh_bytes.saturating_add(slot.fluid_mesh_bytes);
        true
    }

    pub(super) fn replace_fluid_mesh_assets(
        &mut self,
        coord: IVec3,
        meshes: &mut Assets<Mesh>,
        replacements: &mut Vec<(FluidId, Mesh)>,
        fluid_mesh_bytes: usize,
    ) -> bool {
        let Some(slot) = self.active.get_mut(&coord) else {
            return false;
        };
        let Some(terrain_mesh_count) = fluid_mesh_start(slot) else {
            return false;
        };
        if slot.fluid_ids.len() != replacements.len()
            || slot
                .fluid_ids
                .iter()
                .zip(replacements.iter())
                .any(|(existing, (replacement, _))| existing != replacement)
        {
            return false;
        }

        let fluid_handles = &slot.meshes[terrain_mesh_count..];
        if fluid_handles.iter().any(|handle| !meshes.contains(handle)) {
            return false;
        }

        for (handle, (_, replacement)) in fluid_handles.iter().zip(replacements.drain(..)) {
            let mut existing = meshes
                .get_mut(handle)
                .expect("fluid mesh handle was checked before replacement");
            *existing = replacement;
        }

        slot.mesh_bytes = slot
            .mesh_bytes
            .saturating_sub(slot.fluid_mesh_bytes)
            .saturating_add(fluid_mesh_bytes);
        slot.fluid_mesh_bytes = fluid_mesh_bytes;
        true
    }

    pub(super) fn detach_terrain_render_allocation(
        &mut self,
        coord: IVec3,
    ) -> Option<DetachedRenderAllocationParts> {
        let slot = self.active.get_mut(&coord)?;
        let terrain_mesh_count = fluid_mesh_start(slot)?;
        let terrain_entity_count = slot.entities.len().checked_sub(slot.fluid_ids.len())?;

        let entities = slot.entities.drain(..terrain_entity_count).collect();
        let meshes = slot.meshes.drain(..terrain_mesh_count).collect();
        slot.mesh_keys
            .drain(..terrain_mesh_count)
            .for_each(drop);
        slot.mesh_bytes = slot.fluid_mesh_bytes;

        Some(DetachedRenderAllocationParts { entities, meshes })
    }

    pub(super) fn append_terrain_render_allocation(
        &mut self,
        coord: IVec3,
        entities: Vec<Entity>,
        mesh_handles: Vec<Handle<Mesh>>,
        mesh_keys: Vec<ChunkMeshKey>,
        terrain_mesh_bytes: usize,
    ) {
        debug_assert_eq!(mesh_handles.len(), mesh_keys.len());
        debug_assert!(
            mesh_keys
                .iter()
                .all(|key| matches!(key, ChunkMeshKey::Terrain { .. }))
        );

        let slot = self
            .active
            .get_mut(&coord)
            .expect("terrain allocation append requires an active chunk render allocation");

        let mut combined_entities = entities;
        combined_entities.append(&mut slot.entities);
        slot.entities = combined_entities;

        let mut combined_meshes = mesh_handles;
        combined_meshes.append(&mut slot.meshes);
        slot.meshes = combined_meshes;

        let mut combined_keys = mesh_keys;
        combined_keys.append(&mut slot.mesh_keys);
        slot.mesh_keys = combined_keys;

        slot.mesh_bytes = terrain_mesh_bytes.saturating_add(slot.fluid_mesh_bytes);
    }

    pub(super) fn detach_fluid_render_allocation(
        &mut self,
        coord: IVec3,
    ) -> Option<DetachedRenderAllocationParts> {
        let slot = self.active.get_mut(&coord)?;
        let terrain_mesh_count = fluid_mesh_start(slot)?;
        let fluid_entity_count = slot.fluid_ids.len();
        let terrain_entity_count = slot.entities.len().checked_sub(fluid_entity_count)?;

        let entities = slot.entities.split_off(terrain_entity_count);
        let meshes = slot.meshes.split_off(terrain_mesh_count);
        slot.mesh_keys.truncate(terrain_mesh_count);
        slot.mesh_bytes = slot.mesh_bytes.saturating_sub(slot.fluid_mesh_bytes);
        slot.fluid_mesh_bytes = 0;
        slot.fluid_ids.clear();

        Some(DetachedRenderAllocationParts { entities, meshes })
    }

    pub(super) fn append_fluid_render_allocation(
        &mut self,
        coord: IVec3,
        entities: Vec<Entity>,
        mesh_handles: Vec<Handle<Mesh>>,
        fluid_ids: Vec<FluidId>,
        fluid_mesh_bytes: usize,
    ) {
        debug_assert_eq!(entities.len(), fluid_ids.len());
        debug_assert_eq!(mesh_handles.len(), fluid_ids.len());

        let slot = self
            .active
            .get_mut(&coord)
            .expect("fluid allocation append requires an active chunk render allocation");
        slot.mesh_keys
            .extend(fluid_ids.iter().copied().map(ChunkMeshKey::Fluid));
        slot.entities.extend(entities);
        slot.meshes.extend(mesh_handles);
        slot.mesh_bytes = slot.mesh_bytes.saturating_add(fluid_mesh_bytes);
        slot.fluid_mesh_bytes = fluid_mesh_bytes;
        slot.fluid_ids = fluid_ids;
    }

    pub(super) fn insert(&mut self, coord: IVec3, allocation: ChunkRenderAllocation) {
        if self.active.insert(coord, allocation).is_none() {
            self.bump_membership_revision();
        }
    }

    fn clear(&mut self, meshes: &mut Assets<Mesh>) {
        let had_active_allocations = !self.active.is_empty();
        for (_, slot) in self.active.drain() {
            for handle in slot.meshes {
                let _ = meshes.remove(&handle);
            }
        }

        if had_active_allocations {
            self.bump_membership_revision();
        }
    }

    fn bump_membership_revision(&mut self) {
        self.membership_revision = self
            .membership_revision
            .checked_add(1)
            .expect("chunk render pool membership revision exhausted");
    }
}

fn fluid_mesh_start(slot: &ChunkRenderAllocation) -> Option<usize> {
    if slot.mesh_keys.len() != slot.meshes.len() {
        return None;
    }

    let start = slot.meshes.len().checked_sub(slot.fluid_ids.len())?;
    if slot.mesh_keys[start..]
        .iter()
        .zip(&slot.fluid_ids)
        .any(|(key, fluid_id)| *key != ChunkMeshKey::Fluid(*fluid_id))
    {
        return None;
    }

    Some(start)
}

pub(super) fn retire_render_allocation_parts(
    commands: &mut Commands,
    entities: Vec<Entity>,
    mesh_handles: Vec<Handle<Mesh>>,
) {
    for entity in entities {
        commands.entity(entity).despawn();
    }

    if mesh_handles.is_empty() {
        return;
    }

    commands.queue(move |world: &mut World| {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        for mesh_handle in mesh_handles {
            let _ = meshes.remove(&mesh_handle);
        }
    });
}

pub(crate) fn retire_chunk_render_allocation(
    commands: &mut Commands,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
) {
    let Some((entities, mesh_handles)) = render_pool.take(coord) else {
        return;
    };

    retire_render_allocation_parts(commands, entities, mesh_handles);
}

pub(crate) fn clear_chunk_render_pool(
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_pool: ResMut<ChunkRenderPool>,
) {
    render_pool.clear(&mut meshes);
}
