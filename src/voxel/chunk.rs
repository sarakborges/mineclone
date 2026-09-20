use std::sync::{Arc, OnceLock};

use bevy::prelude::*;

use super::{
    cell::VoxelCell,
    fluid::FluidCell,
    light::{BlockLight, VoxelLight},
};

pub const CHUNK_SIZE: usize = 16;
const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;
pub(crate) const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIZE;
const BOUNDARY_FACE_COUNT: usize = 6;
const FLUID_FRONTIER_WORDS: usize = CHUNK_VOLUME.div_ceil(u64::BITS as usize);
const FLUID_SPREAD_TARGETS: [IVec3; 5] = [
    IVec3::NEG_Y,
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Z,
    IVec3::NEG_Z,
];
const NEGATIVE_X_FACE: usize = 0;
const POSITIVE_X_FACE: usize = 1;
const NEGATIVE_Y_FACE: usize = 2;
const POSITIVE_Y_FACE: usize = 3;
const NEGATIVE_Z_FACE: usize = 4;
const POSITIVE_Z_FACE: usize = 5;

fn shared_empty_blocks() -> Arc<[Option<VoxelCell>]> {
    static EMPTY_BLOCKS: OnceLock<Arc<[Option<VoxelCell>]>> = OnceLock::new();
    Arc::clone(EMPTY_BLOCKS.get_or_init(|| Arc::from(vec![None; CHUNK_VOLUME])))
}

fn shared_empty_fluids() -> Arc<[Option<FluidCell>]> {
    static EMPTY_FLUIDS: OnceLock<Arc<[Option<FluidCell>]>> = OnceLock::new();
    Arc::clone(EMPTY_FLUIDS.get_or_init(|| Arc::from(vec![None; CHUNK_VOLUME])))
}

#[derive(Component, Clone)]
pub struct VoxelChunk {
    blocks: Arc<[Option<VoxelCell>]>,
    fluids: Arc<[Option<FluidCell>]>,
    light: Arc<[VoxelLight]>,
    block_count: usize,
    fluid_count: usize,
    fluid_frontier_sources: Arc<[u64; FLUID_FRONTIER_WORDS]>,
    boundary_content_counts: [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: [u16; BOUNDARY_FACE_COUNT],
}

pub(crate) struct VoxelChunkContentMut<'a> {
    blocks: &'a mut [Option<VoxelCell>],
    fluids: &'a mut [Option<FluidCell>],
    block_count: &'a mut usize,
    fluid_count: &'a mut usize,
    fluid_frontier_sources: &'a mut [u64; FLUID_FRONTIER_WORDS],
    boundary_content_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
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
            &mut *self.fluid_frontier_sources,
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
            &mut *self.fluid_frontier_sources,
            &mut *self.boundary_content_counts,
            &mut *self.boundary_fluid_counts,
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
            blocks: shared_empty_blocks(),
            fluids: shared_empty_fluids(),
            light: Arc::from(vec![VoxelLight::DARK; CHUNK_VOLUME]),
            block_count: 0,
            fluid_count: 0,
            fluid_frontier_sources: Arc::new([0; FLUID_FRONTIER_WORDS]),
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

    pub(crate) fn visit_potential_fluid_frontier_sources(
        &self,
        mut visit: impl FnMut(IVec3, FluidCell),
    ) {
        for (word_index, &word) in self.fluid_frontier_sources.iter().enumerate() {
            let mut remaining = word;
            while remaining != 0 {
                let bit = remaining.trailing_zeros() as usize;
                let voxel_index = word_index * u64::BITS as usize + bit;
                if voxel_index >= CHUNK_VOLUME {
                    break;
                }
                let fluid = self.fluids[voxel_index]
                    .expect("fluid frontier source metadata must point to a fluid voxel");
                let (x, y, z) = coordinates(voxel_index);
                visit(IVec3::new(x as i32, y as i32, z as i32), fluid);
                remaining &= remaining - 1;
            }
        }
    }

    pub(crate) fn visit_dynamic_fluid_cells(
        &self,
        mut visit: impl FnMut(IVec3, FluidCell),
    ) {
        if self.fluid_count == 0 {
            return;
        }

        for (voxel_index, fluid) in self.fluids.iter().copied().enumerate() {
            let Some(fluid) = fluid else {
                continue;
            };
            if fluid.is_source() {
                continue;
            }
            let (x, y, z) = coordinates(voxel_index);
            visit(IVec3::new(x as i32, y as i32, z as i32), fluid);
        }
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
        let fluid_frontier_sources = Arc::make_mut(&mut self.fluid_frontier_sources);
        let mut content = VoxelChunkContentMut {
            blocks,
            fluids,
            block_count: &mut self.block_count,
            fluid_count: &mut self.fluid_count,
            fluid_frontier_sources,
            boundary_content_counts: &mut self.boundary_content_counts,
            boundary_fluid_counts: &mut self.boundary_fluid_counts,
        };
        edit(&mut content)
    }

    pub(crate) fn set_block(&mut self, x: usize, y: usize, z: usize, block: Option<VoxelCell>) {
        let blocks = Arc::make_mut(&mut self.blocks);
        let fluid_frontier_sources = Arc::make_mut(&mut self.fluid_frontier_sources);
        set_block_in_storage(
            blocks,
            self.fluids.as_ref(),
            &mut self.block_count,
            fluid_frontier_sources,
            &mut self.boundary_content_counts,
            x,
            y,
            z,
            block,
        );
    }

    pub(crate) fn set_fluid(&mut self, x: usize, y: usize, z: usize, fluid: Option<FluidCell>) {
        let fluids = Arc::make_mut(&mut self.fluids);
        let fluid_frontier_sources = Arc::make_mut(&mut self.fluid_frontier_sources);
        set_fluid_in_storage(
            self.blocks.as_ref(),
            fluids,
            &mut self.fluid_count,
            fluid_frontier_sources,
            &mut self.boundary_content_counts,
            &mut self.boundary_fluid_counts,
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

    pub(crate) fn rebuild_empty_light_columns(&mut self, sky_by_column: &[u8; CHUNK_AREA]) {
        debug_assert!(self.is_empty(), "empty light rebuild requires an empty chunk");
        let lights = Arc::make_mut(&mut self.light);

        for (light, &sky) in lights[..CHUNK_AREA].iter_mut().zip(sky_by_column) {
            *light = VoxelLight::new_hsi(sky, BlockLight::DARK);
        }
        for y in 1..CHUNK_SIZE {
            lights.copy_within(0..CHUNK_AREA, y * CHUNK_AREA);
        }
    }

    pub(crate) fn clear_light(&mut self) {
        Arc::make_mut(&mut self.light).fill(VoxelLight::DARK);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "block mutation keeps chunk occupancy metadata updates atomic"
)]
fn set_block_in_storage(
    blocks: &mut [Option<VoxelCell>],
    fluids: &[Option<FluidCell>],
    block_count: &mut usize,
    fluid_frontier_sources: &mut [u64; FLUID_FRONTIER_WORDS],
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
    refresh_fluid_frontier_sources_near(blocks, fluids, fluid_frontier_sources, x, y, z);
}

#[expect(
    clippy::too_many_arguments,
    reason = "fluid mutation keeps occupancy and boundary metadata updates atomic"
)]
fn set_fluid_in_storage(
    blocks: &[Option<VoxelCell>],
    fluids: &mut [Option<FluidCell>],
    fluid_count: &mut usize,
    fluid_frontier_sources: &mut [u64; FLUID_FRONTIER_WORDS],
    boundary_content_counts: &mut [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: &mut [u16; BOUNDARY_FACE_COUNT],
    x: usize,
    y: usize,
    z: usize,
    fluid: Option<FluidCell>,
) {
    let index = index(x, y, z);
    let previous_fluid = fluids[index];
    let had_fluid = previous_fluid.is_some();
    let had_content = had_fluid || blocks[index].is_some();
    let has_fluid = fluid.is_some();
    let has_content = has_fluid || blocks[index].is_some();

    if had_fluid != has_fluid {
        adjust_total_count(fluid_count, has_fluid);
        adjust_boundary_counts(boundary_fluid_counts, x, y, z, has_fluid);
    }
    if had_content != has_content {
        adjust_boundary_counts(boundary_content_counts, x, y, z, has_content);
    }

    fluids[index] = fluid;
    refresh_fluid_frontier_sources_near(blocks, fluids, fluid_frontier_sources, x, y, z);
}

fn refresh_fluid_frontier_sources_near(
    blocks: &[Option<VoxelCell>],
    fluids: &[Option<FluidCell>],
    sources: &mut [u64; FLUID_FRONTIER_WORDS],
    x: usize,
    y: usize,
    z: usize,
) {
    let changed = IVec3::new(x as i32, y as i32, z as i32);
    refresh_fluid_frontier_source(blocks, fluids, sources, changed);

    for offset in FLUID_SPREAD_TARGETS {
        let source = changed - offset;
        if in_bounds(source.x, source.y, source.z) {
            refresh_fluid_frontier_source(blocks, fluids, sources, source);
        }
    }
}

fn refresh_fluid_frontier_source(
    blocks: &[Option<VoxelCell>],
    fluids: &[Option<FluidCell>],
    sources: &mut [u64; FLUID_FRONTIER_WORDS],
    source: IVec3,
) {
    let source_index = index(source.x as usize, source.y as usize, source.z as usize);
    let should_track = fluids[source_index].is_some()
        && FLUID_SPREAD_TARGETS.iter().any(|offset| {
            let target = source + *offset;
            if !in_bounds(target.x, target.y, target.z) {
                return true;
            }
            let target_index = index(target.x as usize, target.y as usize, target.z as usize);
            blocks[target_index].is_none() && fluids[target_index].is_none()
        });

    let word = source_index / u64::BITS as usize;
    let mask = 1_u64 << (source_index % u64::BITS as usize);
    if should_track {
        sources[word] |= mask;
    } else {
        sources[word] &= !mask;
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

fn coordinates(index: usize) -> (usize, usize, usize) {
    let y = index / CHUNK_AREA;
    let layer_index = index % CHUNK_AREA;
    let z = layer_index / CHUNK_SIZE;
    let x = layer_index % CHUNK_SIZE;
    (x, y, z)
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
    fn empty_chunks_share_content_storage_but_not_light() {
        let first = VoxelChunk::empty();
        let second = VoxelChunk::empty();

        assert!(Arc::ptr_eq(&first.blocks, &second.blocks));
        assert!(Arc::ptr_eq(&first.fluids, &second.fluids));
        assert!(!Arc::ptr_eq(&first.light, &second.light));
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
    fn block_and_fluid_can_share_a_voxel() {
        let mut chunk = VoxelChunk::empty();
        let block = VoxelCell::new("stone", Default::default());
        let fluid = FluidCell::source(0, 8);

        chunk.set_block(2, 3, 4, Some(block));
        chunk.set_fluid(2, 3, 4, Some(fluid));

        assert_eq!(chunk.cell_at(2, 3, 4), Some(block));
        assert_eq!(chunk.fluid_at(2, 3, 4), Some(fluid));
        assert!(!chunk.is_empty());
        assert!(chunk.has_fluid());
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
    fn fluid_frontier_source_metadata_tracks_only_potential_sources() {
        let mut chunk = VoxelChunk::empty();
        let fluid = FluidCell::source(0, 8);
        let source = IVec3::new(5, 5, 5);
        chunk.set_fluid(source.x as usize, source.y as usize, source.z as usize, Some(fluid));

        let mut sources = Vec::new();
        chunk.visit_potential_fluid_frontier_sources(|position, _| sources.push(position));
        assert_eq!(sources, vec![source]);

        for offset in FLUID_SPREAD_TARGETS {
            let target = source + offset;
            chunk.set_block(
                target.x as usize,
                target.y as usize,
                target.z as usize,
                Some(VoxelCell::new("stone", Default::default())),
            );
        }

        sources.clear();
        chunk.visit_potential_fluid_frontier_sources(|position, _| sources.push(position));
        assert!(sources.is_empty());

        let reopened = source + IVec3::X;
        chunk.set_block(
            reopened.x as usize,
            reopened.y as usize,
            reopened.z as usize,
            None,
        );
        chunk.visit_potential_fluid_frontier_sources(|position, _| sources.push(position));
        assert_eq!(sources, vec![source]);
    }

    #[test]
    fn boundary_fluid_stays_a_potential_frontier_source() {
        let mut chunk = VoxelChunk::empty();
        let fluid = FluidCell::source(0, 8);
        let source = IVec3::new(0, 5, 5);
        chunk.set_fluid(source.x as usize, source.y as usize, source.z as usize, Some(fluid));

        for offset in [IVec3::NEG_Y, IVec3::X, IVec3::Z, IVec3::NEG_Z] {
            let target = source + offset;
            chunk.set_block(
                target.x as usize,
                target.y as usize,
                target.z as usize,
                Some(VoxelCell::new("stone", Default::default())),
            );
        }

        let mut sources = Vec::new();
        chunk.visit_potential_fluid_frontier_sources(|position, _| sources.push(position));
        assert_eq!(sources, vec![source]);
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
        assert_eq!(
            chunk.fluid_at(last as i32, last as i32, last as i32),
            Some(fluid)
        );
        assert!(chunk.boundary_has_content(IVec3::NEG_X));
        assert!(chunk.boundary_has_content(IVec3::X));
        assert!(chunk.boundary_has_fluid(IVec3::X));
    }

    #[test]
    fn empty_light_columns_repeat_across_all_layers() {
        let mut chunk = VoxelChunk::empty();
        let mut sky = [VoxelLight::MAX_LEVEL; CHUNK_AREA];
        sky[3 + 5 * CHUNK_SIZE] = 7;

        chunk.rebuild_empty_light_columns(&sky);

        for y in 0..CHUNK_SIZE {
            assert_eq!(chunk.light_at(3, y as i32, 5).sky(), 7);
            assert_eq!(chunk.light_at(3, y as i32, 5).block(), 0);
        }
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
