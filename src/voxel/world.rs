mod persistence;

use std::{collections::BTreeSet, sync::Arc};

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::content::layer::{LayerFace, LayerRegistry};

use super::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, VoxelChunk},
    chunk_archive::ArchivedChunk,
    coordinates::{chunk_coord_from_world, split_world_position},
    fluid::FluidCell,
    layer::LayerCell,
    light::VoxelLight,
};

#[derive(Resource, Default, Clone)]
pub struct VoxelWorld {
    chunks: HashMap<IVec3, VoxelChunk>,
    loaded_chunk_columns: HashMap<IVec2, BTreeSet<i32>>,
    archived_chunks: HashMap<IVec3, Arc<ArchivedChunk>>,
    persistent_chunks: HashSet<IVec3>,
    chunk_content_revisions: HashMap<IVec3, u64>,
    next_chunk_content_revision: u64,
    chunk_mesh_revisions: HashMap<IVec3, u64>,
    next_chunk_mesh_revision: u64,
    block_content_revision: u64,
}

impl VoxelWorld {
    pub fn insert_chunk(&mut self, coord: IVec3, chunk: VoxelChunk) {
        assert!(coord.y >= 0, "chunk Y cannot be negative: {}", coord.y);
        assert!(
            !self.has_resident_or_persisted_chunk(coord),
            "worldgen cannot overwrite a resident or persisted chunk: {coord:?}"
        );
        self.chunks.insert(coord, chunk);
        self.track_loaded_chunk(coord);
        self.bump_block_content_revision();
        self.bump_chunk_content_revision(coord);
        self.bump_chunk_mesh_revision(coord);
    }

    pub fn chunk(&self, coord: IVec3) -> Option<&VoxelChunk> {
        if coord.y < 0 {
            return None;
        }

        self.chunks.get(&coord)
    }

    pub(crate) fn chunk_content_revision(&self, coord: IVec3) -> Option<u64> {
        self.chunk(coord)?;
        Some(
            *self
                .chunk_content_revisions
                .get(&coord)
                .unwrap_or_else(|| panic!("loaded chunk content revision should exist at {coord:?}")),
        )
    }

    #[cfg(test)]
    pub(crate) fn chunk_with_mesh_revision(&self, coord: IVec3) -> Option<(&VoxelChunk, u64)> {
        let chunk = self.chunk(coord)?;
        let revision = *self
            .chunk_mesh_revisions
            .get(&coord)
            .unwrap_or_else(|| panic!("loaded chunk mesh revision should exist at {coord:?}"));
        Some((chunk, revision))
    }

    #[cfg(test)]
    pub(crate) fn chunk_mesh_revision(&self, coord: IVec3) -> Option<u64> {
        self.chunk_with_mesh_revision(coord)
            .map(|(_, revision)| revision)
    }

    pub(crate) fn block_content_revision(&self) -> u64 {
        self.block_content_revision
    }

    pub(crate) fn loaded_chunk_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.chunks.keys().copied()
    }

    pub(crate) fn loaded_chunk_coords_below(
        &self,
        coord: IVec3,
    ) -> impl Iterator<Item = IVec3> + '_ {
        self.loaded_chunk_columns
            .get(&coord.xz())
            .into_iter()
            .flat_map(move |ys| {
                ys.range(..coord.y)
                    .copied()
                    .map(move |y| IVec3::new(coord.x, y, coord.z))
            })
    }

    pub fn archive_chunk(&mut self, coord: IVec3) {
        let Some(chunk) = self.chunks.remove(&coord) else {
            return;
        };
        self.untrack_loaded_chunk(coord);
        let removed_content_revision = self.chunk_content_revisions.remove(&coord);
        debug_assert!(
            removed_content_revision.is_some(),
            "archived loaded chunk should have a content revision: {coord:?}"
        );
        let removed_mesh_revision = self.chunk_mesh_revisions.remove(&coord);
        debug_assert!(
            removed_mesh_revision.is_some(),
            "archived loaded chunk should have a mesh revision: {coord:?}"
        );
        self.bump_block_content_revision();

        if self.persistent_chunks.contains(&coord) {
            self.archived_chunks
                .insert(coord, Arc::new(ArchivedChunk::from_chunk(&chunk)));
        }
    }

    pub fn restore_chunk(&mut self, coord: IVec3) -> bool {
        if self.chunks.contains_key(&coord) {
            return true;
        }

        let Some(archived) = self.archived_chunks.remove(&coord) else {
            return false;
        };

        self.chunks.insert(coord, archived.restore());
        self.track_loaded_chunk(coord);
        self.bump_block_content_revision();
        self.bump_chunk_content_revision(coord);
        self.bump_chunk_mesh_revision(coord);
        true
    }

    pub fn has_resident_or_persisted_chunk(&self, coord: IVec3) -> bool {
        coord.y >= 0
            && (self.chunks.contains_key(&coord) || self.archived_chunks.contains_key(&coord))
    }

    pub fn cell_at(&self, world_position: IVec3) -> Option<VoxelCell> {
        if world_position.y < 0 {
            return None;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);

        self.chunks
            .get(&chunk_coord)?
            .cell_at(local_position.x, local_position.y, local_position.z)
    }

    pub(crate) fn layers_at(
        &self,
        world_position: IVec3,
    ) -> &[super::layer::AttachedLayer] {
        if world_position.y < 0 {
            return &[];
        }

        let (chunk_coord, local_position) = split_world_position(world_position);
        self.chunks.get(&chunk_coord).map_or(&[], |chunk| {
            chunk.layers_at(local_position.x, local_position.y, local_position.z)
        })
    }

    pub fn fluid_at(&self, world_position: IVec3) -> Option<FluidCell> {
        if world_position.y < 0 {
            return None;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);

        self.chunks.get(&chunk_coord)?.fluid_at(
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
        let chunk = self.chunks.get(&chunk_coord)?;

        chunk.sample_local(local_position.x, local_position.y, local_position.z)
    }

    pub(crate) fn light_at(&self, world_position: IVec3) -> VoxelLight {
        if world_position.y < 0 {
            return VoxelLight::DARK;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);
        let Some(chunk) = self.chunks.get(&chunk_coord) else {
            return VoxelLight::DARK;
        };

        chunk.light_at(local_position.x, local_position.y, local_position.z)
    }

    pub(in crate::voxel) fn set_light_at_deferred_mesh_revision(
        &mut self,
        chunk_coord: IVec3,
        local_position: IVec3,
        light: VoxelLight,
    ) -> bool {
        debug_assert!(local_position.x >= 0 && local_position.x < CHUNK_SIZE as i32);
        debug_assert!(local_position.y >= 0 && local_position.y < CHUNK_SIZE as i32);
        debug_assert!(local_position.z >= 0 && local_position.z < CHUNK_SIZE as i32);

        let Some(chunk) = self.chunks.get_mut(&chunk_coord) else {
            return false;
        };

        chunk.set_light(
            local_position.x as usize,
            local_position.y as usize,
            local_position.z as usize,
            light,
        )
    }

    pub(in crate::voxel) fn commit_deferred_light_mesh_revisions(
        &mut self,
        coords: impl IntoIterator<Item = IVec3>,
    ) {
        for coord in coords {
            self.bump_chunk_mesh_revision(coord);
        }
    }

    pub(crate) fn rebuild_chunk_light(
        &mut self,
        coord: IVec3,
        light_at: impl FnMut(usize, usize, usize, Option<VoxelCell>, Option<FluidCell>) -> VoxelLight,
    ) -> bool {
        {
            let Some(chunk) = self.chunks.get_mut(&coord) else {
                return false;
            };
            chunk.rebuild_light(light_at);
        }
        self.bump_chunk_mesh_revision(coord);
        true
    }

    pub(crate) fn rebuild_empty_chunk_light_columns(
        &mut self,
        coord: IVec3,
        sky_by_column: &[u8; CHUNK_SIZE * CHUNK_SIZE],
    ) -> bool {
        {
            let Some(chunk) = self.chunks.get_mut(&coord) else {
                return false;
            };
            chunk.rebuild_empty_light_columns(sky_by_column);
        }
        self.bump_chunk_mesh_revision(coord);
        true
    }

    pub(crate) fn clear_chunk_light(&mut self, coord: IVec3) -> bool {
        {
            let Some(chunk) = self.chunks.get_mut(&coord) else {
                return false;
            };
            chunk.clear_light();
        }
        self.bump_chunk_mesh_revision(coord);
        true
    }

    pub fn is_loaded_at(&self, world_position: IVec3) -> bool {
        if world_position.y < 0 {
            return false;
        }

        self.chunks
            .contains_key(&chunk_coord_from_world(world_position))
    }

    pub(crate) fn highest_loaded_world_y_in_column(
        &self,
        world_x: i32,
        world_z: i32,
    ) -> Option<i32> {
        let horizontal_chunk = chunk_coord_from_world(IVec3::new(world_x, 0, world_z)).xz();
        let highest_chunk_y = self
            .loaded_chunk_columns
            .get(&horizontal_chunk)?
            .last()
            .copied()?;

        Some((highest_chunk_y + 1) * CHUNK_SIZE as i32 - 1)
    }

    #[cfg(test)]
    pub fn set_block_at(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<IVec3> {
        self.set_block_at_with_previous(world_position, block)
            .map(|(chunk_coord, _)| chunk_coord)
    }

    pub(crate) fn set_block_at_with_previous(
        &mut self,
        world_position: IVec3,
        block: Option<VoxelCell>,
    ) -> Option<(IVec3, Option<VoxelCell>)> {
        if world_position.y < 0 {
            return None;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);
        let previous_block;
        let block_changed;

        {
            let chunk = self.chunks.get_mut(&chunk_coord)?;
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
            chunk.set_block(x, y, z, block);

            if block.is_some() {
                chunk.set_fluid(x, y, z, None);
            }
        }

        self.persistent_chunks.insert(chunk_coord);
        if block_changed {
            self.bump_block_content_revision();
        }
        self.bump_chunk_content_revision(chunk_coord);
        self.bump_chunk_mesh_revision(chunk_coord);
        Some((chunk_coord, previous_block))
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
            let chunk = self.chunks.get_mut(&chunk_coord)?;
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

        self.persistent_chunks.insert(chunk_coord);
        self.bump_chunk_content_revision(chunk_coord);
        self.bump_chunk_mesh_revision(chunk_coord);
        Some(chunk_coord)
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
            let chunk = self.chunks.get_mut(&chunk_coord)?;
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

        self.persistent_chunks.insert(chunk_coord);
        self.bump_chunk_content_revision(chunk_coord);
        self.bump_chunk_mesh_revision(chunk_coord);
        Some(chunk_coord)
    }

    pub(crate) fn set_fluid_at(
        &mut self,
        world_position: IVec3,
        fluid: Option<FluidCell>,
    ) -> Option<IVec3> {
        self.set_fluid_at_internal(world_position, fluid, true)
    }

    /// Apply deterministic generated-fluid convergence without promoting the
    /// target chunk to authoritative persistent state. A persisted chunk is
    /// never eligible for this path; its future fluid changes belong to the
    /// runtime scheduler.
    pub(crate) fn set_derived_fluid_at(
        &mut self,
        world_position: IVec3,
        fluid: Option<FluidCell>,
    ) -> Option<IVec3> {
        if world_position.y < 0 {
            return None;
        }
        let chunk_coord = chunk_coord_from_world(world_position);
        if self.persistent_chunks.contains(&chunk_coord) {
            return None;
        }
        self.set_fluid_at_internal(world_position, fluid, false)
    }

    pub(crate) fn derived_fluid_chunk_is_mutable(&self, coord: IVec3) -> bool {
        coord.y >= 0
            && self.chunks.contains_key(&coord)
            && !self.persistent_chunks.contains(&coord)
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
            let chunk = self.chunks.get_mut(&chunk_coord)?;
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
            self.persistent_chunks.insert(chunk_coord);
        }
        self.bump_chunk_content_revision(chunk_coord);
        self.bump_chunk_mesh_revision(chunk_coord);
        Some(chunk_coord)
    }

    pub fn is_solid(&self, world_position: IVec3) -> bool {
        self.cell_at(world_position).is_some()
    }

    pub fn block_id_at(&self, world_position: IVec3) -> Option<&'static str> {
        self.cell_at(world_position).map(|cell| cell.block_id)
    }

    fn bump_block_content_revision(&mut self) {
        self.block_content_revision = self
            .block_content_revision
            .checked_add(1)
            .expect("block content revision counter exhausted");
    }

    fn bump_chunk_content_revision(&mut self, coord: IVec3) {
        debug_assert!(
            self.chunks.contains_key(&coord),
            "content revision bump requires a loaded chunk: {coord:?}"
        );
        self.next_chunk_content_revision = self
            .next_chunk_content_revision
            .checked_add(1)
            .expect("chunk content revision counter exhausted");
        self.chunk_content_revisions
            .insert(coord, self.next_chunk_content_revision);
    }

    fn bump_chunk_mesh_revision(&mut self, coord: IVec3) {
        debug_assert!(
            self.chunks.contains_key(&coord),
            "mesh revision bump requires a loaded chunk: {coord:?}"
        );
        self.next_chunk_mesh_revision = self
            .next_chunk_mesh_revision
            .checked_add(1)
            .expect("chunk mesh revision counter exhausted");
        self.chunk_mesh_revisions
            .insert(coord, self.next_chunk_mesh_revision);
    }

    fn track_loaded_chunk(&mut self, coord: IVec3) {
        self.loaded_chunk_columns
            .entry(coord.xz())
            .or_default()
            .insert(coord.y);
    }

    fn untrack_loaded_chunk(&mut self, coord: IVec3) {
        let horizontal = coord.xz();
        let Some(ys) = self.loaded_chunk_columns.get_mut(&horizontal) else {
            return;
        };

        ys.remove(&coord.y);
        if ys.is_empty() {
            self.loaded_chunk_columns.remove(&horizontal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loaded_column_height_tracks_insert_and_archive() {
        let mut world = VoxelWorld::default();
        let low = IVec3::new(2, 1, -3);
        let high = IVec3::new(2, 4, -3);
        let world_x = low.x * CHUNK_SIZE as i32;
        let world_z = low.z * CHUNK_SIZE as i32;

        world.insert_chunk(low, VoxelChunk::empty());
        world.insert_chunk(high, VoxelChunk::empty());
        assert_eq!(
            world.highest_loaded_world_y_in_column(world_x, world_z),
            Some((high.y + 1) * CHUNK_SIZE as i32 - 1),
        );

        world.archive_chunk(high);
        assert_eq!(
            world.highest_loaded_world_y_in_column(world_x, world_z),
            Some((low.y + 1) * CHUNK_SIZE as i32 - 1),
        );

        world.archive_chunk(low);
        assert_eq!(world.highest_loaded_world_y_in_column(world_x, world_z), None);
    }

    #[test]
    fn loaded_chunk_coords_below_are_column_local_and_sorted() {
        let mut world = VoxelWorld::default();
        for coord in [
            IVec3::new(2, 0, -3),
            IVec3::new(2, 2, -3),
            IVec3::new(2, 4, -3),
            IVec3::new(3, 1, -3),
        ] {
            world.insert_chunk(coord, VoxelChunk::empty());
        }

        assert_eq!(
            world
                .loaded_chunk_coords_below(IVec3::new(2, 4, -3))
                .collect::<Vec<_>>(),
            vec![IVec3::new(2, 0, -3), IVec3::new(2, 2, -3)]
        );
    }

    #[test]
    fn unmodified_generated_chunk_is_dropped_when_archived() {
        let mut world = VoxelWorld::default();
        let coord = IVec3::new(3, 2, -4);
        world.insert_chunk(coord, VoxelChunk::empty());

        world.archive_chunk(coord);

        assert!(!world.has_resident_or_persisted_chunk(coord));
        assert!(!world.restore_chunk(coord));
    }

    #[test]
    fn modified_chunk_is_persisted_when_archived() {
        let mut world = VoxelWorld::default();
        let coord = IVec3::ZERO;
        world.insert_chunk(coord, VoxelChunk::empty());
        world.set_block_at(
            IVec3::new(1, 1, 1),
            Some(VoxelCell::new("stone", Default::default())),
        );

        world.archive_chunk(coord);

        assert!(world.has_resident_or_persisted_chunk(coord));
        assert!(world.restore_chunk(coord));
    }

    #[test]
    fn layer_mutation_tracks_content_without_changing_block_revision() {
        let mut world = VoxelWorld::default();
        let coord = IVec3::ZERO;
        let position = IVec3::new(1, 2, 3);
        world.insert_chunk(coord, VoxelChunk::empty());
        world.set_block_at(
            position,
            Some(VoxelCell::new("stone", Default::default())),
        );

        let mut registry = LayerRegistry::default();
        registry.insert(crate::content::layer::LayerDefinition {
            id: "asteria:test_layer".to_owned(),
            texture: "textures/test.png".to_owned(),
            tint: crate::content::block::BlockTint::None,
            faces: vec![LayerFace::Top],
            offset: 1.0 / 1024.0,
            alpha_cutoff: Some(0.5),
            alpha_blend: false,
            casts_shadow: false,
        });

        let block_revision = world.block_content_revision();
        assert_eq!(
            world.add_layer_at(
                position,
                LayerFace::Top,
                LayerCell::new("asteria:test_layer", Default::default()),
                &registry,
            ),
            Some(coord),
        );
        assert_eq!(world.block_content_revision(), block_revision);
        assert_eq!(world.layers_at(position).len(), 1);
    }

    #[test]
    fn identical_block_mutation_is_ignored() {
        let mut world = VoxelWorld::default();
        let coord = IVec3::ZERO;
        let position = IVec3::new(1, 2, 3);
        let cell = VoxelCell::new("stone", Default::default());
        world.insert_chunk(coord, VoxelChunk::empty());

        assert_eq!(world.set_block_at(position, Some(cell)), Some(coord));
        assert_eq!(world.set_block_at(position, Some(cell)), None);
    }

    #[test]
    fn detailed_block_mutation_returns_previous_cell() {
        let mut world = VoxelWorld::default();
        let coord = IVec3::ZERO;
        let position = IVec3::new(1, 2, 3);
        let first = VoxelCell::new("stone", Default::default());
        let second = VoxelCell::new("dirt", Default::default());
        world.insert_chunk(coord, VoxelChunk::empty());
        world.set_block_at(position, Some(first));

        assert_eq!(
            world.set_block_at_with_previous(position, Some(second)),
            Some((coord, Some(first))),
        );
    }

    #[test]
    fn chunk_content_revision_ignores_lighting_but_tracks_voxel_content() {
        let mut world = VoxelWorld::default();
        let coord = IVec3::ZERO;
        let position = IVec3::new(1, 2, 3);
        world.insert_chunk(coord, VoxelChunk::empty());
        let inserted_content = world
            .chunk_content_revision(coord)
            .expect("chunk should have a content revision");
        let inserted_mesh = world.chunk_mesh_revision(coord).expect("chunk should be loaded");

        assert!(world.clear_chunk_light(coord));
        assert_eq!(world.chunk_content_revision(coord), Some(inserted_content));
        assert!(world.chunk_mesh_revision(coord).expect("chunk should be loaded") > inserted_mesh);

        world.set_block_at(position, Some(VoxelCell::new("stone", Default::default())));
        assert!(
            world
                .chunk_content_revision(coord)
                .expect("chunk should have a content revision")
                > inserted_content
        );
    }

    #[test]
    fn mesh_revision_tracks_resident_chunk_mutations_and_restore() {
        let mut world = VoxelWorld::default();
        let coord = IVec3::ZERO;
        let position = IVec3::new(1, 2, 3);
        world.insert_chunk(coord, VoxelChunk::empty());
        let inserted = world.chunk_mesh_revision(coord).expect("chunk should be loaded");

        world.set_block_at(position, Some(VoxelCell::new("stone", Default::default())));
        let edited = world.chunk_mesh_revision(coord).expect("chunk should be loaded");
        assert!(edited > inserted);

        world.archive_chunk(coord);
        assert_eq!(world.chunk_content_revision(coord), None);
        assert_eq!(world.chunk_mesh_revision(coord), None);
        assert!(world.restore_chunk(coord));
        let restored = world.chunk_mesh_revision(coord).expect("chunk should be restored");
        assert!(restored > edited);
    }
}
