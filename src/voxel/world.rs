use std::collections::{BTreeSet, HashMap, HashSet};

use bevy::prelude::*;

use super::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, VoxelChunk},
    chunk_archive::ArchivedChunk,
    coordinates::{chunk_coord_from_world, split_world_position},
    fluid::FluidCell,
    light::VoxelLight,
};

#[derive(Resource, Default)]
pub struct VoxelWorld {
    chunks: HashMap<IVec3, VoxelChunk>,
    loaded_chunk_columns: HashMap<IVec2, BTreeSet<i32>>,
    archived_chunks: HashMap<IVec3, ArchivedChunk>,
    generated_chunks: HashSet<IVec3>,
    dirty_chunks: HashSet<IVec3>,
    chunk_mesh_revisions: HashMap<IVec3, u64>,
    next_chunk_mesh_revision: u64,
}

impl VoxelWorld {
    pub fn insert_chunk(&mut self, coord: IVec3, chunk: VoxelChunk) {
        assert!(coord.y >= 0, "chunk Y cannot be negative: {}", coord.y);
        assert!(
            !self.generated_chunks.contains(&coord),
            "worldgen cannot overwrite an already generated chunk: {coord:?}"
        );

        self.generated_chunks.insert(coord);
        self.chunks.insert(coord, chunk);
        self.track_loaded_chunk(coord);
        self.bump_chunk_mesh_revision(coord);
    }

    pub fn chunk(&self, coord: IVec3) -> Option<&VoxelChunk> {
        if coord.y < 0 {
            return None;
        }

        self.chunks.get(&coord)
    }

    pub(crate) fn chunk_with_mesh_revision(&self, coord: IVec3) -> Option<(&VoxelChunk, u64)> {
        let chunk = self.chunk(coord)?;
        let revision = *self
            .chunk_mesh_revisions
            .get(&coord)
            .unwrap_or_else(|| panic!("loaded chunk mesh revision should exist at {coord:?}"));
        Some((chunk, revision))
    }

    pub(crate) fn chunk_mesh_revision(&self, coord: IVec3) -> Option<u64> {
        self.chunk_with_mesh_revision(coord)
            .map(|(_, revision)| revision)
    }

    pub(crate) fn loaded_chunk_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.chunks.keys().copied()
    }

    pub fn archive_chunk(&mut self, coord: IVec3) {
        let Some(chunk) = self.chunks.remove(&coord) else {
            return;
        };
        self.untrack_loaded_chunk(coord);
        let removed_revision = self.chunk_mesh_revisions.remove(&coord);
        debug_assert!(
            removed_revision.is_some(),
            "archived loaded chunk should have a mesh revision: {coord:?}"
        );

        if self.dirty_chunks.contains(&coord) {
            self.archived_chunks
                .insert(coord, ArchivedChunk::from_chunk(&chunk));
        } else {
            self.generated_chunks.remove(&coord);
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
        self.bump_chunk_mesh_revision(coord);
        true
    }

    pub fn has_generated_chunk(&self, coord: IVec3) -> bool {
        coord.y >= 0 && self.generated_chunks.contains(&coord)
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

    pub(crate) fn set_light_at(&mut self, world_position: IVec3, light: VoxelLight) -> bool {
        if world_position.y < 0 {
            return false;
        }

        let (chunk_coord, local_position) = split_world_position(world_position);
        let changed = {
            let Some(chunk) = self.chunks.get_mut(&chunk_coord) else {
                return false;
            };

            chunk.set_light(
                local_position.x as usize,
                local_position.y as usize,
                local_position.z as usize,
                light,
            )
        };
        if changed {
            self.bump_chunk_mesh_revision(chunk_coord);
        }
        changed
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
            chunk.set_block(x, y, z, block);

            if block.is_some() {
                chunk.set_fluid(x, y, z, None);
            }
        }

        self.dirty_chunks.insert(chunk_coord);
        self.bump_chunk_mesh_revision(chunk_coord);
        Some((chunk_coord, previous_block))
    }

    pub(crate) fn set_fluid_at(
        &mut self,
        world_position: IVec3,
        fluid: Option<FluidCell>,
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
        self.dirty_chunks.insert(chunk_coord);
        self.bump_chunk_mesh_revision(chunk_coord);
        Some(chunk_coord)
    }

    pub fn is_solid(&self, world_position: IVec3) -> bool {
        self.cell_at(world_position).is_some()
    }

    pub fn block_id_at(&self, world_position: IVec3) -> Option<&'static str> {
        self.cell_at(world_position).map(|cell| cell.block_id)
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
        assert_eq!(world.chunk_mesh_revision(coord), None);
        assert!(world.restore_chunk(coord));
        let restored = world.chunk_mesh_revision(coord).expect("chunk should be restored");
        assert!(restored > edited);
    }
}
