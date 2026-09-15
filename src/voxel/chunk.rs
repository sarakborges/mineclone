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
    blocks: Box<[Option<VoxelCell>]>,
    fluids: Box<[Option<FluidCell>]>,
    light: Box<[VoxelLight]>,
    block_count: usize,
    fluid_count: usize,
    boundary_content_counts: [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: [u16; BOUNDARY_FACE_COUNT],
}

impl VoxelChunk {
    pub fn empty() -> Self {
        Self {
            blocks: vec![None; CHUNK_VOLUME].into_boxed_slice(),
            fluids: vec![None; CHUNK_VOLUME].into_boxed_slice(),
            light: vec![VoxelLight::DARK; CHUNK_VOLUME].into_boxed_slice(),
            block_count: 0,
            fluid_count: 0,
            boundary_content_counts: [0; BOUNDARY_FACE_COUNT],
            boundary_fluid_counts: [0; BOUNDARY_FACE_COUNT],
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

    pub(crate) fn set_block(&mut self, x: usize, y: usize, z: usize, block: Option<VoxelCell>) {
        let index = index(x, y, z);
        let had_block = self.blocks[index].is_some();
        let had_content = had_block || self.fluids[index].is_some();
        let has_block = block.is_some();
        let has_content = has_block || self.fluids[index].is_some();

        if had_block != has_block {
            adjust_total_count(&mut self.block_count, has_block);
        }
        if had_content != has_content {
            adjust_boundary_counts(
                &mut self.boundary_content_counts,
                x,
                y,
                z,
                has_content,
            );
        }

        self.blocks[index] = block;
    }

    pub(crate) fn set_fluid(&mut self, x: usize, y: usize, z: usize, fluid: Option<FluidCell>) {
        let index = index(x, y, z);
        let had_fluid = self.fluids[index].is_some();
        let had_content = had_fluid || self.blocks[index].is_some();
        let has_fluid = fluid.is_some();
        let has_content = has_fluid || self.blocks[index].is_some();

        if had_fluid != has_fluid {
            adjust_total_count(&mut self.fluid_count, has_fluid);
            adjust_boundary_counts(
                &mut self.boundary_fluid_counts,
                x,
                y,
                z,
                has_fluid,
            );
        }
        if had_content != has_content {
            adjust_boundary_counts(
                &mut self.boundary_content_counts,
                x,
                y,
                z,
                has_content,
            );
        }

        self.fluids[index] = fluid;
    }

    pub(crate) fn set_light(&mut self, x: usize, y: usize, z: usize, light: VoxelLight) -> bool {
        let index = index(x, y, z);
        if self.light[index] == light {
            return false;
        }
        self.light[index] = light;
        true
    }

    pub(crate) fn clear_light(&mut self) {
        self.light.fill(VoxelLight::DARK);
    }
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

        chunk.set_block(1, 1, 1, Some(VoxelCell::new("stone")));
        assert!(!chunk.is_empty());

        chunk.set_block(1, 1, 1, None);
        assert!(chunk.is_empty());
    }

    #[test]
    fn boundary_content_tracks_union_of_blocks_and_fluids() {
        let mut chunk = VoxelChunk::empty();
        let last = CHUNK_SIZE - 1;
        let fluid = FluidCell::source(0);

        chunk.set_fluid(0, last, 3, Some(fluid));
        assert!(chunk.boundary_has_content(IVec3::NEG_X));
        assert!(chunk.boundary_has_content(IVec3::Y));
        assert!(chunk.boundary_has_fluid(IVec3::NEG_X));

        chunk.set_block(0, last, 3, Some(VoxelCell::new("stone")));
        chunk.set_fluid(0, last, 3, None);
        assert!(chunk.boundary_has_content(IVec3::NEG_X));
        assert!(!chunk.boundary_has_fluid(IVec3::NEG_X));

        chunk.set_block(0, last, 3, None);
        assert!(!chunk.boundary_has_content(IVec3::NEG_X));
        assert!(!chunk.boundary_has_content(IVec3::Y));
    }
}
