use std::sync::Arc;

use bevy::prelude::*;

use super::{cell::VoxelCell, fluid::FluidCell, light::VoxelLight};

pub const CHUNK_SIZE: usize = 16;
const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;
pub(crate) const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIZE;
const BOUNDARY_FACE_COUNT: usize = 6;
const NEGATIVE_X_FACE: usize = 0;
const POSITIVE_X_FACE: usize = 1;
const NEGATIVE_Y_FACE: usize = 2;
const POSITIVE_Y_FACE: usize = 3;
const NEGATIVE_Z_FACE: usize = 4;
const POSITIVE_Z_FACE: usize = 5;

#[derive(Component, Clone)]
pub struct VoxelChunk {
    blocks: Arc<[Option<VoxelCell>]>,
    fluids: Arc<[Option<FluidCell>]>,
    light: Arc<[VoxelLight]>,
    block_count: usize,
    fluid_count: usize,
    boundary_content_counts: [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: [u16; BOUNDARY_FACE_COUNT],
    boundary_dynamic_fluid_counts: [u16; BOUNDARY_FACE_COUNT],
}

pub(crate) struct VoxelChunkContentMut<'a> {
    blocks: &'a mut [Option<VoxelCell>],
    fluids: &'a mut [Option<FluidCell>],
    block_count: &'a mut usize,
    fluid_count: &'a mut usize,
    boundary_content_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
    boundary_dynamic_fluid_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
}

impl VoxelChunkContentMut<'_> {
    pub(crate) fn set_block(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block: Option<VoxelCell>,
    ) {
        set_block_in_storage(
            &mut *self.blocks,
            &*self.fluids,
            &mut *self.block_count,
            &mut *self.boundary_content_counts,
            x,
            y,
            z,
            block,
        );
    }

    pub(crate) fn set_fluid(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        fluid: Option<FluidCell>,
    ) {
        set_fluid_in_storage(
            &*self.blocks,
            &mut *self.fluids,
            &mut *self.fluid_count,
            &mut *self.boundary_content_counts,
            &mut *self.boundary_fluid_counts,
            &mut *self.boundary_dynamic_fluid_counts,
            x,
            y,
            z,
            fluid,
        );
    }
}

impl VoxelChunk {
    pub fn empty() -> Self {
        Self {
            blocks: Arc::from(vec![None; CHUNK_VOLUME]),
            fluids: Arc::from(vec![None; CHUNK_VOLUME]),
            light: Arc::from(vec![VoxelLight::DARK; CHUNK_VOLUME]),
            block_count: 0,
            fluid_count: 0,
            boundary_content_counts: [0; BOUNDARY_FACE_COUNT],
            boundary_fluid_counts: [0; BOUNDARY_FACE_COUNT],
            boundary_dynamic_fluid_counts: [0; BOUNDARY_FACE_COUNT],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.block_count == 0 && self.fluid_count == 0
    }

    pub(crate) fn has_fluid(&self) -> bool {
        self.fluid_count > 0
    }

    pub(crate) fn boundary_has_content(&self, outward: IVec3) -> bool {
        boundary_face_index(outward)
            .is_some_and(|face| self.boundary_content_counts[face] > 0)
    }

    pub(crate) fn boundary_has_fluid(&self, outward: IVec3) -> bool {
        boundary_face_index(outward)
            .is_some_and(|face| self.boundary_fluid_counts[face] > 0)
    }

    pub(crate) fn boundary_dynamic_fluid_count(&self, outward: IVec3) -> usize {
        boundary_face_index(outward)
            .map(|face| self.boundary_dynamic_fluid_counts[face] as usize)
            .unwrap_or(0)
    }

    pub fn cell_at(&self, x: i32, y: i32, z: i32) -> Option<VoxelCell> {
        if !in_bounds(x, y, z) {
            return None;
        }

        self.blocks[index(x as usize, y as usize, z as usize)]
    }

    pub fn fluid_at(&self, x: i32, y: i32, z: i32) -> Option<FluidCell> {
        if !in_bounds(x, y, z) {
            return None;
        }

        self.fluids[index(x as usize, y as usize, z as usize)]
    }

    pub(crate) fn light_at(&self, x: i32, y: i32, z: i32) -> VoxelLight {
        if !in_bounds(x, y, z) {
            return VoxelLight::DARK;
        }

        self.light[index(x as usize, y as usize, z as usize)]
    }

    pub(crate) fn sample_local(
        &self,
        x: i32,
        y: i32,
        z: i32,
    ) -> Option<(Option<VoxelCell>, Option<FluidCell>, VoxelLight)> {
        if !in_bounds(x, y, z) {
            return None;
        }

        let index = index(x as usize, y as usize, z as usize);
        Some((self.blocks[index], self.fluids[index], self.light[index]))
    }

    pub(crate) fn edit_content<R>(
        &mut self,
        edit: impl FnOnce(&mut VoxelChunkContentMut<'_>) -> R,
    ) -> R {
        let blocks = Arc::make_mut(&mut self.blocks);
        let fluids = Arc::make_mut(&mut self.fluids);
        let mut content = VoxelChunkContentMut {
            blocks,
            fluids,
            block_count: &mut self.block_count,
            fluid_count: &mut self.fluid_count,
            boundary_content_counts: &mut self.boundary_content_counts,
            boundary_fluid_counts: &mut self.boundary_fluid_counts,
            boundary_dynamic_fluid_counts: &mut self.boundary_dynamic_fluid_counts,
        };
        edit(&mut content)
    }

    pub(crate) fn set_block(&mut self, x: usize, y: usize, z: usize, block: Option<VoxelCell>) {
        let blocks = Arc::make_mut(&mut self.blocks);
        set_block_in_storage(
            blocks,
            self.fluids.as_ref(),
            &mut self.block_count,
            &mut self.boundary_content_counts,
            x,
            y,
            z,
            block,
        );
    }

    pub(crate) fn set_fluid(&mut self, x: usize, y: usize, z: usize, fluid: Option<FluidCell>) {
        let fluids = Arc::make_mut(&mut self.fluids);
        set_fluid_in_storage(
            self.blocks.as_ref(),
            fluids,
            &mut self.fluid_count,
            &mut self.boundary_content_counts,
            &mut self.boundary_fluid_counts,
            &mut self.boundary_dynamic_fluid_counts,
            x,
            y,
            z,
            fluid,
        );
    }

    pub(crate) fn set_light(&mut self, x: usize, y: usize, z: usize, light: VoxelLight) -> bool {
        let index = index(x, y, z);
        if self.light[index] == light {
            return false;
        }
        Arc::make_mut(&mut self.light)[index] = light;
        true
    }

    pub(crate) fn rebuild_light(
        &mut self,
        mut light_at: impl FnMut(
            usize,
            usize,
            usize,
            Option<VoxelCell>,
            Option<FluidCell>,
        ) -> VoxelLight,
    ) {
        let blocks = &self.blocks;
        let fluids = &self.fluids;
        let lights = Arc::make_mut(&mut self.light);

        for y in (0..CHUNK_SIZE).rev() {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let index = index(x, y, z);
                    lights[index] = light_at(x, y, z, blocks[index], fluids[index]);
                }
            }
        }
    }

    pub(crate) fn clear_light(&mut self) {
        Arc::make_mut(&mut self.light).fill(VoxelLight::DARK);
    }
}

fn set_block_in_storage(
    blocks: &mut [Option<VoxelCell>],
    fluids: &[Option<FluidCell>],
    block_count: &mut usize,
    boundary_content_counts: &mut [u16; BOUNDARY_FACE_COUNT],
    x: usize,
    y: usize,
    z: usize,
    block: Option<VoxelCell>,
) {
    let index = index(x, y, z);
    let had_block = blocks[index].is_some();
    let had_content = had_block || fluids[index].is_some();
    let has_block = block.is_some();
    let has_content = has_block || fluids[index].is_some();

    if had_block != has_block {
        adjust_total_count(block_count, has_block);
    }
    if had_content != has_content {
        adjust_boundary_counts(boundary_content_counts, x, y, z, has_content);
    }

    blocks[index] = block;
}

fn set_fluid_in_storage(
    blocks: &[Option<VoxelCell>],
    fluids: &mut [Option<FluidCell>],
    fluid_count: &mut usize,
    boundary_content_counts: &mut [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: &mut [u16; BOUNDARY_FACE_COUNT],
    boundary_dynamic_fluid_counts: &mut [u16; BOUNDARY_FACE_COUNT],
    x: usize,
    y: usize,
    z: usize,
    fluid: Option<FluidCell>,
) {
    let index = index(x, y, z);
    let previous_fluid = fluids[index];
    let had_fluid = previous_fluid.is_some();
    let had_dynamic_fluid = previous_fluid.is_some_and(|cell| !cell.is_source());
    let had_content = had_fluid || blocks[index].is_some();
    let has_fluid = fluid.is_some();
    let has_dynamic_fluid = fluid.is_some_and(|cell| !cell.is_source());
    let has_content = has_fluid || blocks[index].is_some();

    if had_fluid != has_fluid {
        adjust_total_count(fluid_count, has_fluid);
        adjust_boundary_counts(boundary_fluid_counts, x, y, z, has_fluid);
    }
    if had_dynamic_fluid != has_dynamic_fluid {
        adjust_boundary_counts(
            boundary_dynamic_fluid_counts,
            x,
            y,
            z,
            has_dynamic_fluid,
        );
    }
    if had_content != has_content {
        adjust_boundary_counts(boundary_content_counts, x, y, z, has_content);
    }

    fluids[index] = fluid;
}

fn adjust_total_count(count: &mut usize, added: bool) {
    if added {
        *count += 1;
    } else {
        *count = count
            .checked_sub(1)
            .expect("chunk occupancy count cannot underflow");
    }
}

fn adjust_boundary_counts(
    counts: &mut [u16; BOUNDARY_FACE_COUNT],
    x: usize,
    y: usize,
    z: usize,
    added: bool,
) {
    let last = CHUNK_SIZE - 1;

    if x == 0 {
        adjust_boundary_count(&mut counts[NEGATIVE_X_FACE], added);
    }
    if x == last {
        adjust_boundary_count(&mut counts[POSITIVE_X_FACE], added);
    }
    if y == 0 {
        adjust_boundary_count(&mut counts[NEGATIVE_Y_FACE], added);
    }
    if y == last {
        adjust_boundary_count(&mut counts[POSITIVE_Y_FACE], added);
    }
    if z == 0 {
        adjust_boundary_count(&mut counts[NEGATIVE_Z_FACE], added);
    }
    if z == last {
        adjust_boundary_count(&mut counts[POSITIVE_Z_FACE], added);
    }
}

fn adjust_boundary_count(count: &mut u16, added: bool) {
    if added {
        *count = count
            .checked_add(1)
            .expect("chunk boundary occupancy count cannot overflow");
    } else {
        *count = count
            .checked_sub(1)
            .expect("chunk boundary occupancy count cannot underflow");
    }
}

fn boundary_face_index(outward: IVec3) -> Option<usize> {
    if outward == IVec3::NEG_X {
        Some(NEGATIVE_X_FACE)
    } else if outward == IVec3::X {
        Some(POSITIVE_X_FACE)
    } else if outward == IVec3::NEG_Y {
        Some(NEGATIVE_Y_FACE)
    } else if outward == IVec3::Y {
        Some(POSITIVE_Y_FACE)
    } else if outward == IVec3::NEG_Z {
        Some(NEGATIVE_Z_FACE)
    } else if outward == IVec3::Z {
        Some(POSITIVE_Z_FACE)
    } else {
        None
    }
}

fn in_bounds(x: i32, y: i32, z: i32) -> bool {
    x >= 0
        && y >= 0
        && z >= 0
        && x < CHUNK_SIZE as i32
        && y < CHUNK_SIZE as i32
        && z < CHUNK_SIZE as i32
}

fn index(x: usize, y: usize, z: usize) -> usize {
    x + z * CHUNK_SIZE + y * CHUNK_AREA
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_chunk_uses_constant_time_occupancy_counts() {
        let mut chunk = VoxelChunk::empty();
        assert!(chunk.is_empty());
        assert!(!chunk.has_fluid());

        chunk.set_block(1, 1, 1, Some(VoxelCell::new("stone", Default::default())));
        assert!(!chunk.is_empty());

        chunk.set_block(1, 1, 1, None);
        assert!(chunk.is_empty());
    }

    #[test]
    fn boundary_content_tracks_union_of_blocks_and_fluids() {
        let mut chunk = VoxelChunk::empty();
        let last = CHUNK_SIZE - 1;
        let fluid = FluidCell::source(0, 8);

        chunk.set_fluid(0, last, 3, Some(fluid));
        assert!(chunk.boundary_has_content(IVec3::NEG_X));
        assert!(chunk.boundary_has_content(IVec3::Y));
        assert!(chunk.boundary_has_fluid(IVec3::NEG_X));

        chunk.set_block(
            0,
            last,
            3,
            Some(VoxelCell::new("stone", Default::default())),
        );
        chunk.set_fluid(0, last, 3, None);
        assert!(chunk.boundary_has_content(IVec3::NEG_X));
        assert!(!chunk.boundary_has_fluid(IVec3::NEG_X));

        chunk.set_block(0, last, 3, None);
        assert!(!chunk.boundary_has_content(IVec3::NEG_X));
        assert!(!chunk.boundary_has_content(IVec3::Y));
    }

    #[test]
    fn boundary_dynamic_fluid_tracks_source_transitions() {
        let mut chunk = VoxelChunk::empty();
        let last = CHUNK_SIZE - 1;

        chunk.set_fluid(0, last, 3, Some(FluidCell::source(0, 8)));
        assert!(chunk.boundary_has_fluid(IVec3::NEG_X));
        assert_eq!(chunk.boundary_dynamic_fluid_count(IVec3::NEG_X), 0);
        assert_eq!(chunk.boundary_dynamic_fluid_count(IVec3::Y), 0);

        chunk.set_fluid(0, last, 3, Some(FluidCell::spreading(0, 7, 1)));
        assert_eq!(chunk.boundary_dynamic_fluid_count(IVec3::NEG_X), 1);
        assert_eq!(chunk.boundary_dynamic_fluid_count(IVec3::Y), 1);

        chunk.set_fluid(0, last, 3, Some(FluidCell::source(0, 8)));
        assert_eq!(chunk.boundary_dynamic_fluid_count(IVec3::NEG_X), 0);
        assert_eq!(chunk.boundary_dynamic_fluid_count(IVec3::Y), 0);
    }

    #[test]
    fn local_sample_resolves_all_voxel_channels_once() {
        let mut chunk = VoxelChunk::empty();
        let cell = VoxelCell::new("stone", Default::default());
        let fluid = FluidCell::source(0, 8);
        let light = VoxelLight::new_hsi(12, Default::default());
        chunk.set_block(1, 2, 3, Some(cell));
        chunk.set_fluid(4, 5, 6, Some(fluid));
        chunk.set_light(7, 8, 9, light);

        assert_eq!(chunk.sample_local(1, 2, 3).unwrap().0, Some(cell));
        assert_eq!(chunk.sample_local(4, 5, 6).unwrap().1, Some(fluid));
        assert_eq!(chunk.sample_local(7, 8, 9).unwrap().2, light);
        assert!(chunk.sample_local(-1, 0, 0).is_none());
    }

    #[test]
    fn batched_content_edits_preserve_metadata() {
        let mut chunk = VoxelChunk::empty();
        let last = CHUNK_SIZE - 1;
        let block = VoxelCell::new("stone", Default::default());
        let fluid = FluidCell::spreading(0, 7, 1);

        chunk.edit_content(|content| {
            content.set_block(0, 0, 0, Some(block));
            content.set_fluid(last, last, last, Some(fluid));
        });

        assert_eq!(chunk.cell_at(0, 0, 0), Some(block));
        assert_eq!(chunk.fluid_at(last as i32, last as i32, last as i32), Some(fluid));
        assert!(chunk.boundary_has_content(IVec3::NEG_X));
        assert!(chunk.boundary_has_content(IVec3::X));
        assert!(chunk.boundary_has_fluid(IVec3::X));
        assert_eq!(chunk.boundary_dynamic_fluid_count(IVec3::X), 1);
    }

    #[test]
    fn chunk_clone_shares_storage_until_mutated() {
        let mut chunk = VoxelChunk::empty();
        chunk.set_block(1, 2, 3, Some(VoxelCell::new("stone", Default::default())));
        chunk.set_fluid(4, 5, 6, Some(FluidCell::source(0, 8)));
        chunk.set_light(7, 8, 9, VoxelLight::new_hsi(12, Default::default()));

        let mut clone = chunk.clone();
        assert!(Arc::ptr_eq(&chunk.blocks, &clone.blocks));
        assert!(Arc::ptr_eq(&chunk.fluids, &clone.fluids));
        assert!(Arc::ptr_eq(&chunk.light, &clone.light));

        clone.set_block(1, 2, 3, None);
        clone.set_fluid(4, 5, 6, None);
        clone.set_light(7, 8, 9, VoxelLight::DARK);

        assert!(!Arc::ptr_eq(&chunk.blocks, &clone.blocks));
        assert!(!Arc::ptr_eq(&chunk.fluids, &clone.fluids));
        assert!(!Arc::ptr_eq(&chunk.light, &clone.light));
        assert!(chunk.cell_at(1, 2, 3).is_some());
        assert!(chunk.fluid_at(4, 5, 6).is_some());
        assert_ne!(chunk.light_at(7, 8, 9), VoxelLight::DARK);
    }
}
