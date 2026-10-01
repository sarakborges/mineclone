mod block_revision;
mod content_revision;
mod object_revision;
mod persistence;
mod resident;
mod resident_index;

use bevy::prelude::*;

use crate::content::{
    layer::{LayerFace, LayerRegistry},
    object::ObjectRegistry,
};

use self::{
    block_revision::BlockRevisionState,
    content_revision::ContentRevisionState,
    object_revision::ObjectRevisionState,
    persistence::ChunkPersistenceState,
    resident::ResidentChunkStore,
};
use super::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, ObjectCells, VoxelChunk},
    coordinates::{chunk_coord_from_world, split_world_position},
    fluid::FluidCell,
    layer::LayerCell,
    light::VoxelLight,
    object::ObjectCell,
    revision::{BlockTopologyRevision, ChunkContentRevision},
};

#[derive(Resource, Default, Clone)]
pub struct VoxelWorld {
    resident: ResidentChunkStore,
    persistence: ChunkPersistenceState,
    content_revisions: ContentRevisionState,
    object_revisions: ObjectRevisionState,
    block_revisions: BlockRevisionState,
}

impl VoxelWorld {
    pub fn insert_chunk(&mut self, coord: IVec3, chunk: VoxelChunk) {
        assert!(coord.y >= 0, "chunk Y cannot be negative: {}", coord.y);
        assert!(
            !self.has_resident_or_persisted_chunk(coord),
            "worldgen cannot overwrite a resident or persisted chunk: {coord:?}"
        );
        self.resident.insert(coord, chunk);
        self.block_revisions.mark_topology_changed();
        self.bump_chunk_content_revision(coord);
        self.mark_chunk_objects_changed(coord);
    }

    pub fn chunk(&self, coord: IVec3) -> Option<&VoxelChunk> {
        if coord.y < 0 {
            return None;
        }
        self.resident.get(coord)
    }

    pub(crate) fn chunk_content_revision(&self, coord: IVec3) -> Option<ChunkContentRevision> {
        self.chunk(coord)?;
        Some(
            self.content_revisions
                .revision(coord)
                .unwrap_or_else(|| panic!("loaded chunk content revision should exist at {coord:?}")),
        )
    }

    pub(crate) fn block_topology_revision(&self) -> BlockTopologyRevision {
        self.block_revisions.topology()
    }

    pub(crate) fn object_scene_revision(&self) -> u64 {
        self.object_revisions.scene_revision()
    }

    pub(crate) fn chunk_object_revision(&self, coord: IVec3) -> Option<u64> {
        self.chunk(coord)?;
        Some(
            self.object_revisions
                .chunk_revision(coord)
                .unwrap_or_else(|| panic!("loaded chunk object revision should exist at {coord:?}")),
        )
    }

    pub(crate) fn loaded_chunk_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.resident.coords()
    }

    pub(crate) fn loaded_chunk_coords_below(
        &self,
        coord: IVec3,
    ) -> impl Iterator<Item = IVec3> + '_ {
        self.resident.coords_below(coord)
    }

    pub fn archive_chunk(&mut self, coord: IVec3) {
        let Some(chunk) = self.resident.remove(coord) else {
            return;
        };
        let removed_content_revision = self.content_revisions.remove_chunk(coord);
        debug_assert!(
            removed_content_revision.is_some(),
            "archived loaded chunk should have a content revision: {coord:?}"
        );
        let removed_object_revision = self.object_revisions.remove_chunk(coord);
        debug_assert!(
            removed_object_revision.is_some(),
            "archived loaded chunk should have an object revision: {coord:?}"
        );
        self.block_revisions.mark_topology_changed();
        self.persistence.archive_if_persistent(coord, &chunk);
    }

    pub fn restore_chunk(&mut self, coord: IVec3) -> bool {
        if self.resident.contains(coord) {
            return true;
        }
        let Some(chunk) = self.persistence.restore(coord) else {
            return false;
        };
        self.resident.insert(coord, chunk);
        self.block_revisions.mark_topology_changed();
        self.bump_chunk_content_revision(coord);
        self.mark_chunk_objects_changed(coord);
        true
    }

    pub fn has_resident_or_persisted_chunk(&self, coord: IVec3) -> bool {
        coord.y >= 0 && (self.resident.contains(coord) || self.persistence.has_archived(coord))
    }

    pub fn cell_at(&self, world_position: IVec3) -> Option<VoxelCell> {
        if world_position.y < 0 {
            return None;
        }
        let (chunk_coord, local_position) = split_world_position(world_position);
        self.resident
            .get(chunk_coord)?
            .cell_at(local_position.x, local_position.y, local_position.z)
    }

    pub(crate) fn objects_at(&self, world_position: IVec3) -> &[ObjectCell] {
        if world_position.y < 0 {
            return &[];
        }
        let (chunk_coord, local_position) = split_world_position(world_position);
        self.resident.get(chunk_coord).map_or(&[], |chunk| {
            chunk.objects_at(local_position.x, local_position.y, local_position.z)
        })
    }

    pub(crate) fn object_at(&self, world_position: IVec3) -> Option<ObjectCell> {
        self.objects_at(world_position).first().copied()
    }

    pub(crate) fn layers_at(
        &self,
        world_position: IVec3,
    ) -> &[super::layer::AttachedLayer] {
        if world_position.y < 0 {
            return &[];
        }
        let (chunk_coord, local_position) = split_world_position(world_position);
        self.resident.get(chunk_coord).map_or(&[], |chunk| {
            chunk.layers_at(local_position.x, local_position.y, local_position.z)
        })
    }

    pub fn fluid_at(&self, world_position: IVec3) -> Option<FluidCell> {
        if world_position.y < 0 {
            return None;
        }
        let (chunk_coord, local_position) = split_world_position(world_position);
        self.resident.get(chunk_coord)?.fluid_at(
            local_position.x,
            local_position.y,
            local_position.z,
        )
    }

    pub(crate) fn sample_at(
        &self,
        world_position: IVec3,
    ) -> Option<(Option<VoxelCell>, Option<FluidCell>, VoxelLight)> {
        if world_position.y < 0 {
            return None;
        }
        let (chunk_coord, local_position) = split_world_position(world_position);
        self.resident
            .get(chunk_coord)?
            .sample_local(local_position.x, local_position.y, local_position.z)
    }

    pub(crate) fn light_at(&self, world_position: IVec3) -> VoxelLight {
        if world_position.y < 0 {
            return VoxelLight::DARK;
        }
        let (chunk_coord, local_position) = split_world_position(world_position);
        let Some(chunk) = self.resident.get(chunk_coord) else {
            return VoxelLight::DARK;
        };
        chunk.light_at(local_position.x, local_position.y, local_position.z)
    }

    pub(in crate::voxel) fn set_light_at(
        &mut self,
        chunk_coord: IVec3,
        local_position: IVec3,
        light: VoxelLight,
    ) -> bool {
        debug_assert!(local_position.x >= 0 && local_position.x < CHUNK_SIZE as i32);
        debug_assert!(local_position.y >= 0 && local_position.y < CHUNK_SIZE as i32);
        debug_assert!(local_position.z >= 0 && local_position.z < CHUNK_SIZE as i32);
        let Some(chunk) = self.resident.get_mut(chunk_coord) else {
            return false;
        };
        chunk.set_light(
            local_position.x as usize,
            local_position.y as usize,
            local_position.z as usize,
            light,
        )
    }

    pub(crate) fn rebuild_chunk_light(
        &mut self,
        coord: IVec3,
        light_at: impl FnMut(usize, usize, usize, Option<VoxelCell>, Option<FluidCell>) -> VoxelLight,
    ) -> bool {
        let Some(chunk) = self.resident.get_mut(coord) else {
            return false;
        };
        chunk.rebuild_light(light_at);
        true
    }

    pub(crate) fn rebuild_empty_chunk_light_columns(
        &mut self,
        coord: IVec3,
        sky_by_column: &[u8; CHUNK_SIZE * CHUNK_SIZE],
    ) -> bool {
        let Some(chunk) = self.resident.get_mut(coord) else {
            return false;
        };
        chunk.rebuild_empty_light_columns(coord, sky_by_column);
        true
    }

    #[cfg(test)]
    pub(crate) fn clear_chunk_light(&mut self, coord: IVec3) -> bool {
        let Some(chunk) = self.resident.get_mut(coord) else {
            return false;
        };
        chunk.clear_light();
        true
    }

    pub fn is_loaded_at(&self, world_position: IVec3) -> bool {
        if world_position.y < 0 {
            return false;
        }
        self.resident
            .contains(chunk_coord_from_world(world_position))
    }

    pub(crate) fn highest_loaded_world_y_in_column(
        &self,
        world_x: i32,
        world_z: i32,
    ) -> Option<i32> {
        let horizontal_chunk = chunk_coord_from_world(IVec3::new(world_x, 0, world_z)).xz();
        self.resident.highest_world_y_in_column(horizontal_chunk)
    }

    #[cfg(test)]
    pub fn set_block_at(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<IVec3> {
        self.set_block_at_with_previous(world_position, block)
            .map(|(chunk_coord, _, _)| chunk_coord)
    }

    pub(crate) fn set_block_at_with_previous(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<(IVec3, Option<VoxelCell>, ObjectCells)> {
        if world_position.y < 0 {
            return None;
        }
        let (chunk_coord, local_position) = split_world_position(world_position);
        let previous_block;
        let block_changed;
        let detached_objects;
        {
            let chunk = self.resident.get_mut(chunk_coord)?;
            let x = local_position.x as usize;
            let y = local_position.y as usize;
            let z = local_position.z as usize;
            let (current_block, current_fluid, _) = chunk
                .sample_local(local_position.x, local_position.y, local_position.z)
                .expect("split local block coordinates must stay inside the chunk");
            if current_block == block && (block.is_none() || current_fluid.is_none()) {
                return None;
            }
            previous_block = current_block;
            block_changed = current_block != block;
            detached_objects = chunk.set_block_with_detached_objects(x, y, z, block);
            if block.is_some() {
                chunk.set_fluid(x, y, z, None);
            }
        }
        self.persistence.mark_persistent(chunk_coord);
        if block_changed {
            self.block_revisions.mark_topology_changed();
            self.mark_chunk_objects_changed(chunk_coord);
        }
        self.bump_chunk_content_revision(chunk_coord);
        Some((chunk_coord, previous_block, detached_objects))
    }

    pub(crate) fn add_layer_at(
        &mut self,
        world_position: IVec3,
        face: LayerFace,
        layer: LayerCell,
        registry: &LayerRegistry,
    ) -> Option<IVec3> {
        if world_position.y < 0 {
            return None;
        }
        let definition = registry.get(layer.layer_id)?;
        if !definition.supports_face(face) {
            return None;
        }
        let (chunk_coord, local_position) = split_world_position(world_position);
        let changed = {
            let chunk = self.resident.get_mut(chunk_coord)?;
            chunk.add_layer(
                local_position.x as usize,
                local_position.y as usize,
                local_position.z as usize,
                face,
                layer,
            )
        };
        if !changed {
            return None;
        }
        self.persistence.mark_persistent(chunk_coord);
        self.bump_chunk_content_revision(chunk_coord);
        Some(chunk_coord)
    }

    pub(crate) fn remove_top_layer_at(
        &mut self,
        world_position: IVec3,
        face: LayerFace,
    ) -> Option<(IVec3, &'static str)> {
        let layer_id = self
            .layers_at(world_position)
            .iter()
            .rev()
            .find(|attached| attached.face == face)
            .map(|attached| attached.cell.layer_id)?;
        let chunk = self.remove_layer_at(world_position, face, layer_id)?;
        Some((chunk, layer_id))
    }

    pub(crate) fn remove_layer_at(
        &mut self,
        world_position: IVec3,
        face: LayerFace,
        layer_id: &str,
    ) -> Option<IVec3> {
        if world_position.y < 0 {
            return None;
        }
        let (chunk_coord, local_position) = split_world_position(world_position);
        let changed = {
            let chunk = self.resident.get_mut(chunk_coord)?;
            chunk.remove_layer(
                local_position.x as usize,
                local_position.y as usize,
                local_position.z as usize,
                face,
                layer_id,
            )
        };
        if !changed {
            return None;
        }
        self.persistence.mark_persistent(chunk_coord);
        self.bump_chunk_content_revision(chunk_coord);
        Some(chunk_coord)
    }

    pub(crate) fn set_object_at(
        &mut self,
        support_position: IVec3,
        object: ObjectCell,
        registry: &ObjectRegistry,
    ) -> Option<IVec3> {
        if support_position.y < 0 {
            return None;
        }
        let definition = registry.get(object.object_id)?;
        if !definition.supports_placement_face(object.face) {
            return None;
        }
        let (chunk_coord, local_position) = split_world_position(support_position);
        let changed = {
            let chunk = self.resident.get_mut(chunk_coord)?;
            chunk.set_object(
                local_position.x as usize,
                local_position.y as usize,
                local_position.z as usize,
                object,
            )
        };
        if !changed {
            return None;
        }
        self.persistence.mark_persistent(chunk_coord);
        self.bump_chunk_content_revision(chunk_coord);
        self.mark_chunk_objects_changed(chunk_coord);
        Some(chunk_coord)
    }

    pub(crate) fn remove_object_at(
        &mut self,
        support_position: IVec3,
        object: ObjectCell,
    ) -> Option<(IVec3, ObjectCell)> {
        if support_position.y < 0 {
            return None;
        }
        let (chunk_coord, local_position) = split_world_position(support_position);
        let removed = {
            let chunk = self.resident.get_mut(chunk_coord)?;
            chunk.remove_object(
                local_position.x as usize,
                local_position.y as usize,
                local_position.z as usize,
                object,
            )?
        };
        self.persistence.mark_persistent(chunk_coord);
        self.bump_chunk_content_revision(chunk_coord);
        self.mark_chunk_objects_changed(chunk_coord);
        Some((chunk_coord, removed))
    }

    pub(crate) fn set_fluid_at(
        &mut self,
        world_position: IVec3,
        fluid: Option<FluidCell>,
    ) -> Option<IVec3> {
        self.set_fluid_at_internal(world_position, fluid, true)
    }

    pub(crate) fn set_derived_fluid_at(
        &mut self,
        world_position: IVec3,
        fluid: Option<FluidCell>,
    ) -> Option<IVec3> {
        if world_position.y < 0 {
            return None;
        }
        let chunk_coord = chunk_coord_from_world(world_position);
        if self.persistence.is_persistent(chunk_coord) {
            return None;
        }
        self.set_fluid_at_internal(world_position, fluid, false)
    }

    pub(crate) fn derived_fluid_chunk_is_mutable(&self, coord: IVec3) -> bool {
        coord.y >= 0
            && self.resident.contains(coord)
            && !self.persistence.is_persistent(coord)
    }

    fn set_fluid_at_internal(
        &mut self,
        world_position: IVec3,
        fluid: Option<FluidCell>,
        persistent: bool,
    ) -> Option<IVec3> {
        if world_position.y < 0 {
            return None;
        }
        let (chunk_coord, local_position) = split_world_position(world_position);
        {
            let chunk = self.resident.get_mut(chunk_coord)?;
            let x = local_position.x as usize;
            let y = local_position.y as usize;
            let z = local_position.z as usize;
            let (current_block, current_fluid, _) = chunk
                .sample_local(local_position.x, local_position.y, local_position.z)
                .expect("split local fluid coordinates must stay inside the chunk");
            if fluid.is_some() && current_block.is_some() {
                return None;
            }
            if current_fluid == fluid {
                return None;
            }
            chunk.set_fluid(x, y, z, fluid);
        }
        if persistent {
            self.persistence.mark_persistent(chunk_coord);
        }
        self.bump_chunk_content_revision(chunk_coord);
        Some(chunk_coord)
    }

    pub fn is_solid(&self, world_position: IVec3) -> bool {
        self.cell_at(world_position).is_some()
    }

    pub fn block_id_at(&self, world_position: IVec3) -> Option<&'static str> {
        self.cell_at(world_position).map(|cell| cell.block_id)
    }

    fn mark_chunk_objects_changed(&mut self, coord: IVec3) {
        self.object_revisions.mark_chunk_changed(coord);
    }

    fn bump_chunk_content_revision(&mut self, coord: IVec3) {
        debug_assert!(
            self.resident.contains(coord),
            "content revision bump requires a loaded chunk: {coord:?}"
        );
        self.content_revisions.mark_changed(coord);
    }
}
