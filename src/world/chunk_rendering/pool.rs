use bevy::{platform::collections::HashMap, prelude::*};

use crate::{
    content::fluid::FluidId,
    voxel::{
        block_face::BlockFace,
        fluid_mesh::ChunkFluidMesh,
        meshlet::{ChunkMeshletMask, VoxelMeshPatch, patch_voxel_mesh},
    },
};

use super::spawn::BuiltChunkMesh;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChunkMeshKey {
    TerrainArray {
        alpha_cutoff: Option<u32>,
        alpha_blend: bool,
        casts_shadow: bool,
    },
    TerrainLegacy {
        block_id: &'static str,
        face: BlockFace,
        casts_shadow: bool,
    },
    Layer {
        layer_id: &'static str,
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
    active_column_counts: HashMap<IVec2, usize>,
    total_mesh_bytes: usize,
    membership_revision: u64,
}

impl ChunkRenderPool {
    pub(crate) fn contains(&self, coord: IVec3) -> bool {
        self.active.contains_key(&coord)
    }

    pub(crate) fn contains_column(&self, column: IVec2) -> bool {
        self.active_column_counts.contains_key(&column)
    }

    pub(crate) fn membership_revision(&self) -> u64 {
        self.membership_revision
    }

    pub(crate) fn active_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.active.keys().copied()
    }

    pub(crate) fn active_count(&self) -> usize {
        self.active.len()
    }

    pub(crate) fn mesh_count(&self) -> usize {
        self.active.values().map(|slot| slot.meshes.len()).sum()
    }

    pub(crate) fn entity_count(&self) -> usize {
        self.active.values().map(|slot| slot.entities.len()).sum()
    }

    pub(crate) fn diagnostic_mesh_kind_counts(&self) -> (usize, usize, usize, usize) {
        let mut terrain_array = 0;
        let mut terrain_legacy = 0;
        let mut layers = 0;
        let mut fluids = 0;

        for key in self
            .active
            .values()
            .flat_map(|allocation| allocation.mesh_keys.iter())
        {
            match key {
                ChunkMeshKey::TerrainArray { .. } => terrain_array += 1,
                ChunkMeshKey::TerrainLegacy { .. } => terrain_legacy += 1,
                ChunkMeshKey::Layer { .. } => layers += 1,
                ChunkMeshKey::Fluid(_) => fluids += 1,
            }
        }

        (terrain_array, terrain_legacy, layers, fluids)
    }

    pub(crate) fn mesh_bytes(&self) -> usize {
        self.total_mesh_bytes
    }

    pub(crate) fn diagnostic_recomputed_mesh_bytes(&self) -> usize {
        self.active.values().map(|slot| slot.mesh_bytes).sum()
    }

    pub(crate) fn diagnostic_active_columns_are_consistent(&self) -> bool {
        let mut recomputed = HashMap::<IVec2, usize>::default();
        for coord in self.active.keys() {
            let count = recomputed.entry(coord.xz()).or_default();
            *count = count
                .checked_add(1)
                .expect("diagnostic chunk column count cannot overflow");
        }
        recomputed == self.active_column_counts
    }

    pub(crate) fn mesh_bytes_for(&self, coord: IVec3) -> usize {
        self.active
            .get(&coord)
            .map_or(0, |allocation| allocation.mesh_bytes)
    }

    fn take(&mut self, coord: IVec3) -> Option<(Vec<Entity>, Vec<Handle<Mesh>>)> {
        let slot = self.active.remove(&coord)?;
        replace_aggregated_mesh_bytes(
            &mut self.total_mesh_bytes,
            slot.mesh_bytes,
            0,
        );
        self.remove_active_column(coord.xz());
        self.bump_membership_revision();
        Some((slot.entities, slot.meshes))
    }

    pub(super) fn patch_terrain_mesh_assets(
        &mut self,
        coord: IVec3,
        meshes: &mut Assets<Mesh>,
        replacements: &[BuiltChunkMesh],
        dirty: ChunkMeshletMask,
    ) -> bool {
        let Some(slot) = self.active.get_mut(&coord) else {
            return false;
        };
        let Some(terrain_mesh_count) = fluid_mesh_start(slot) else {
            return false;
        };
        let terrain_keys = &slot.mesh_keys[..terrain_mesh_count];
        if replacements
            .iter()
            .any(|replacement| !terrain_keys.contains(&replacement.key()))
            || slot.meshes[..terrain_mesh_count]
                .iter()
                .any(|handle| !meshes.contains(handle))
        {
            return false;
        }

        let mut patched = Vec::with_capacity(terrain_mesh_count);
        for (index, key) in terrain_keys.iter().copied().enumerate() {
            let handle = &slot.meshes[index];
            let existing = meshes
                .get(handle)
                .expect("terrain mesh handle was checked before meshlet patch");
            let replacement = replacements
                .iter()
                .find(|replacement| replacement.key() == key)
                .map(BuiltChunkMesh::mesh);
            let Some(patch) = patch_voxel_mesh(existing, replacement, dirty) else {
                return false;
            };
            patched.push(patch);
        }

        let previous_mesh_bytes = slot.mesh_bytes;
        let mut terrain_mesh_bytes = 0;
        for (index, patch) in patched.into_iter().enumerate() {
            let handle = &slot.meshes[index];
            match patch {
                VoxelMeshPatch::Unchanged => {
                    terrain_mesh_bytes += mesh_asset_bytes(
                        meshes
                            .get(handle)
                            .expect("unchanged terrain mesh must remain resident"),
                    );
                }
                VoxelMeshPatch::Changed(replacement) => {
                    terrain_mesh_bytes += mesh_asset_bytes(&replacement);
                    *meshes
                        .get_mut(handle)
                        .expect("terrain mesh handle must survive meshlet preflight") = replacement;
                }
            }
        }

        slot.mesh_bytes = terrain_mesh_bytes.saturating_add(slot.fluid_mesh_bytes);
        replace_aggregated_mesh_bytes(
            &mut self.total_mesh_bytes,
            previous_mesh_bytes,
            slot.mesh_bytes,
        );
        true
    }

    pub(super) fn patch_fluid_mesh_assets(
        &mut self,
        coord: IVec3,
        meshes: &mut Assets<Mesh>,
        replacements: &[ChunkFluidMesh],
        dirty: ChunkMeshletMask,
    ) -> bool {
        let Some(slot) = self.active.get_mut(&coord) else {
            return false;
        };
        let Some(terrain_mesh_count) = fluid_mesh_start(slot) else {
            return false;
        };
        if replacements
            .iter()
            .any(|replacement| !slot.fluid_ids.contains(&replacement.fluid_id))
        {
            return false;
        }

        let fluid_handles = &slot.meshes[terrain_mesh_count..];
        if fluid_handles.iter().any(|handle| !meshes.contains(handle)) {
            return false;
        }

        let mut patched = Vec::with_capacity(fluid_handles.len());
        for (index, fluid_id) in slot.fluid_ids.iter().copied().enumerate() {
            let existing = meshes
                .get(&fluid_handles[index])
                .expect("fluid mesh handle was checked before meshlet patch");
            let replacement = replacements
                .iter()
                .find(|replacement| replacement.fluid_id == fluid_id)
                .map(|replacement| &replacement.mesh);
            let Some(patch) = patch_voxel_mesh(existing, replacement, dirty) else {
                return false;
            };
            patched.push(patch);
        }

        let previous_mesh_bytes = slot.mesh_bytes;
        let mut fluid_mesh_bytes = 0;
        for (index, patch) in patched.into_iter().enumerate() {
            let handle = &fluid_handles[index];
            match patch {
                VoxelMeshPatch::Unchanged => {
                    fluid_mesh_bytes += mesh_asset_bytes(
                        meshes
                            .get(handle)
                            .expect("unchanged fluid mesh must remain resident"),
                    );
                }
                VoxelMeshPatch::Changed(replacement) => {
                    fluid_mesh_bytes += mesh_asset_bytes(&replacement);
                    *meshes
                        .get_mut(handle)
                        .expect("fluid mesh handle must survive meshlet preflight") = replacement;
                }
            }
        }

        slot.mesh_bytes = slot
            .mesh_bytes
            .saturating_sub(slot.fluid_mesh_bytes)
            .saturating_add(fluid_mesh_bytes);
        slot.fluid_mesh_bytes = fluid_mesh_bytes;
        replace_aggregated_mesh_bytes(
            &mut self.total_mesh_bytes,
            previous_mesh_bytes,
            slot.mesh_bytes,
        );
        true
    }

    pub(super) fn replace_terrain_mesh_assets(
        &mut self,
        coord: IVec3,
        meshes: &mut Assets<Mesh>,
        replacements: &mut Vec<BuiltChunkMesh>,
        terrain_mesh_bytes: usize,
    ) -> bool {
        let Some(slot) = self.active.get_mut(&coord) else {
            return false;
        };
        let Some(terrain_mesh_count) = fluid_mesh_start(slot) else {
            return false;
        };
        if terrain_mesh_count != replacements.len()
            || !slot.mesh_keys[..terrain_mesh_count]
                .iter()
                .copied()
                .eq(replacements.iter().map(BuiltChunkMesh::key))
            || slot.meshes[..terrain_mesh_count]
                .iter()
                .any(|handle| !meshes.contains(handle))
        {
            return false;
        }

        let previous_mesh_bytes = slot.mesh_bytes;
        for (handle, replacement) in slot.meshes[..terrain_mesh_count]
            .iter()
            .zip(replacements.drain(..))
        {
            let mut existing = meshes
                .get_mut(handle)
                .expect("terrain mesh handle was checked before replacement");
            *existing = replacement.into_mesh();
        }
        slot.mesh_bytes = terrain_mesh_bytes.saturating_add(slot.fluid_mesh_bytes);
        replace_aggregated_mesh_bytes(
            &mut self.total_mesh_bytes,
            previous_mesh_bytes,
            slot.mesh_bytes,
        );
        true
    }

    pub(super) fn replace_fluid_mesh_assets(
        &mut self,
        coord: IVec3,
        meshes: &mut Assets<Mesh>,
        replacements: &mut Vec<ChunkFluidMesh>,
        fluid_mesh_bytes: usize,
    ) -> bool {
        let Some(slot) = self.active.get_mut(&coord) else {
            return false;
        };
        let Some(terrain_mesh_count) = fluid_mesh_start(slot) else {
            return false;
        };
        if !slot
            .fluid_ids
            .iter()
            .copied()
            .eq(replacements.iter().map(|replacement| replacement.fluid_id))
        {
            return false;
        }

        let fluid_handles = &slot.meshes[terrain_mesh_count..];
        if fluid_handles.iter().any(|handle| !meshes.contains(handle)) {
            return false;
        }

        let previous_mesh_bytes = slot.mesh_bytes;
        for (handle, replacement) in fluid_handles.iter().zip(replacements.drain(..)) {
            let mut existing = meshes
                .get_mut(handle)
                .expect("fluid mesh handle was checked before replacement");
            *existing = replacement.mesh;
        }

        slot.mesh_bytes = slot
            .mesh_bytes
            .saturating_sub(slot.fluid_mesh_bytes)
            .saturating_add(fluid_mesh_bytes);
        slot.fluid_mesh_bytes = fluid_mesh_bytes;
        replace_aggregated_mesh_bytes(
            &mut self.total_mesh_bytes,
            previous_mesh_bytes,
            slot.mesh_bytes,
        );
        true
    }

    pub(super) fn detach_terrain_render_allocation(
        &mut self,
        coord: IVec3,
    ) -> Option<DetachedRenderAllocationParts> {
        let slot = self.active.get_mut(&coord)?;
        let previous_mesh_bytes = slot.mesh_bytes;
        let terrain_mesh_count = fluid_mesh_start(slot)?;
        let terrain_entity_count = slot.entities.len().checked_sub(slot.fluid_ids.len())?;

        let entities = slot.entities.drain(..terrain_entity_count).collect();
        let meshes = slot.meshes.drain(..terrain_mesh_count).collect();
        slot.mesh_keys
            .drain(..terrain_mesh_count)
            .for_each(drop);
        slot.mesh_bytes = slot.fluid_mesh_bytes;
        replace_aggregated_mesh_bytes(
            &mut self.total_mesh_bytes,
            previous_mesh_bytes,
            slot.mesh_bytes,
        );

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
                .all(|key| matches!(
                    key,
                    ChunkMeshKey::TerrainArray { .. }
                        | ChunkMeshKey::TerrainLegacy { .. }
                        | ChunkMeshKey::Layer { .. }
                ))
        );

        let slot = self
            .active
            .get_mut(&coord)
            .expect("terrain allocation append requires an active chunk render allocation");
        let previous_mesh_bytes = slot.mesh_bytes;

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
        replace_aggregated_mesh_bytes(
            &mut self.total_mesh_bytes,
            previous_mesh_bytes,
            slot.mesh_bytes,
        );
    }

    pub(super) fn detach_fluid_render_allocation(
        &mut self,
        coord: IVec3,
    ) -> Option<DetachedRenderAllocationParts> {
        let slot = self.active.get_mut(&coord)?;
        let previous_mesh_bytes = slot.mesh_bytes;
        let terrain_mesh_count = fluid_mesh_start(slot)?;
        let fluid_entity_count = slot.fluid_ids.len();
        let terrain_entity_count = slot.entities.len().checked_sub(fluid_entity_count)?;

        let entities = slot.entities.split_off(terrain_entity_count);
        let meshes = slot.meshes.split_off(terrain_mesh_count);
        slot.mesh_keys.truncate(terrain_mesh_count);
        slot.mesh_bytes = slot.mesh_bytes.saturating_sub(slot.fluid_mesh_bytes);
        slot.fluid_mesh_bytes = 0;
        slot.fluid_ids.clear();
        replace_aggregated_mesh_bytes(
            &mut self.total_mesh_bytes,
            previous_mesh_bytes,
            slot.mesh_bytes,
        );

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
        let previous_mesh_bytes = slot.mesh_bytes;
        slot.mesh_keys
            .extend(fluid_ids.iter().copied().map(ChunkMeshKey::Fluid));
        slot.entities.extend(entities);
        slot.meshes.extend(mesh_handles);
        slot.mesh_bytes = slot.mesh_bytes.saturating_add(fluid_mesh_bytes);
        slot.fluid_mesh_bytes = fluid_mesh_bytes;
        slot.fluid_ids = fluid_ids;
        replace_aggregated_mesh_bytes(
            &mut self.total_mesh_bytes,
            previous_mesh_bytes,
            slot.mesh_bytes,
        );
    }

    pub(super) fn insert(&mut self, coord: IVec3, allocation: ChunkRenderAllocation) {
        let allocation_mesh_bytes = allocation.mesh_bytes;
        let previous = self.active.insert(coord, allocation);
        replace_aggregated_mesh_bytes(
            &mut self.total_mesh_bytes,
            previous.as_ref().map_or(0, |slot| slot.mesh_bytes),
            allocation_mesh_bytes,
        );
        if previous.is_none() {
            self.add_active_column(coord.xz());
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

        self.active_column_counts.clear();
        self.total_mesh_bytes = 0;
        if had_active_allocations {
            self.bump_membership_revision();
        }
    }

    fn add_active_column(&mut self, column: IVec2) {
        let count = self.active_column_counts.entry(column).or_default();
        *count = count
            .checked_add(1)
            .expect("chunk render pool column occupancy cannot overflow");
    }

    fn remove_active_column(&mut self, column: IVec2) {
        let remove = {
            let count = self
                .active_column_counts
                .get_mut(&column)
                .expect("active chunk must belong to an active render column");
            *count = count
                .checked_sub(1)
                .expect("chunk render pool column occupancy cannot underflow");
            *count == 0
        };
        if remove {
            self.active_column_counts.remove(&column);
        }
    }

    fn bump_membership_revision(&mut self) {
        self.membership_revision = self
            .membership_revision
            .checked_add(1)
            .expect("chunk render pool membership revision exhausted");
    }
}

fn replace_aggregated_mesh_bytes(total: &mut usize, previous: usize, current: usize) {
    *total = total
        .checked_sub(previous)
        .expect("chunk render pool mesh byte accounting cannot underflow")
        .checked_add(current)
        .expect("chunk render pool mesh byte accounting cannot overflow");
}

fn mesh_asset_bytes(mesh: &Mesh) -> usize {
    mesh.get_vertex_buffer_size()
        + mesh
            .get_index_buffer_bytes()
            .map_or(0, |indices| indices.len())
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

pub(crate) fn retire_chunk_render_allocation_immediately(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
) {
    let Some((entities, mesh_handles)) = render_pool.take(coord) else {
        return;
    };

    for entity in entities {
        commands.entity(entity).despawn();
    }
    for mesh_handle in mesh_handles {
        let _ = meshes.remove(&mesh_handle);
    }
}

pub(crate) fn clear_chunk_render_pool(
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_pool: ResMut<ChunkRenderPool>,
) {
    render_pool.clear(&mut meshes);
}
