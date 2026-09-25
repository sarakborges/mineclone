use std::sync::{Arc, OnceLock};

use bevy::{platform::collections::HashMap, prelude::*};

use crate::content::layer::LayerFace;

use super::{
    cell::VoxelCell,
    fluid::FluidCell,
    layer::{AttachedLayer, LayerCell, MAX_LAYERS_PER_VOXEL},
    light::{BlockLight, VoxelLight},
    object::ObjectCell,
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

#[derive(Clone)]
struct PaletteStorage<T> {
    palette: Vec<T>,
    usage: Vec<u16>,
    indices: Box<[u16]>,
    occupied: Box<[u64; FLUID_FRONTIER_WORDS]>,
}

impl<T> Default for PaletteStorage<T> {
    fn default() -> Self {
        Self {
            palette: Vec::new(),
            usage: Vec::new(),
            indices: vec![0; CHUNK_VOLUME].into_boxed_slice(),
            occupied: Box::new([0; FLUID_FRONTIER_WORDS]),
        }
    }
}

impl<T: Copy + Eq> PaletteStorage<T> {
    fn get(&self, voxel_index: usize) -> Option<T> {
        self.get_ref(voxel_index).copied()
    }

    fn get_ref(&self, voxel_index: usize) -> Option<&T> {
        let palette_index = self.indices[voxel_index];
        (palette_index != 0).then(|| &self.palette[palette_index as usize - 1])
    }

    fn set(&mut self, voxel_index: usize, value: Option<T>) {
        let previous = self.indices[voxel_index];
        let next = match value {
            None => 0,
            Some(value) => {
                let palette_index = self
                    .palette
                    .iter()
                    .position(|candidate| *candidate == value)
                    .or_else(|| self.usage.iter().position(|&usage| usage == 0));

                let palette_index = match palette_index {
                    Some(index) => {
                        if self.usage[index] == 0 {
                            self.palette[index] = value;
                        }
                        index
                    }
                    None => {
                        self.palette.push(value);
                        self.usage.push(0);
                        self.palette.len() - 1
                    }
                };

                u16::try_from(palette_index + 1)
                    .expect("chunk palette cannot exceed u16 index space")
            }
        };

        if previous == next {
            return;
        }

        if previous != 0 {
            let usage = &mut self.usage[previous as usize - 1];
            *usage = usage
                .checked_sub(1)
                .expect("chunk palette usage cannot underflow");
        }
        if next != 0 {
            let usage = &mut self.usage[next as usize - 1];
            *usage = usage
                .checked_add(1)
                .expect("chunk palette usage cannot overflow");
        }
        self.set_palette_index(voxel_index, next);
    }

    fn set_palette_index(&mut self, voxel_index: usize, palette_index: u16) {
        self.indices[voxel_index] = palette_index;
        let word = voxel_index / u64::BITS as usize;
        let mask = 1_u64 << (voxel_index % u64::BITS as usize);
        if palette_index == 0 {
            self.occupied[word] &= !mask;
        } else {
            self.occupied[word] |= mask;
        }
    }

    fn occupied_indices(&self) -> impl Iterator<Item = usize> + '_ {
        let mut word_index = 0_usize;
        let mut remaining = self.occupied[0];

        std::iter::from_fn(move || loop {
            while remaining == 0 {
                word_index += 1;
                if word_index >= self.occupied.len() {
                    return None;
                }
                remaining = self.occupied[word_index];
            }

            let bit = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;
            let voxel_index = word_index * u64::BITS as usize + bit;
            if voxel_index < CHUNK_VOLUME {
                return Some(voxel_index);
            }
        })
    }

    fn visit_occupied_indices(&self, mut visit: impl FnMut(usize)) {
        for voxel_index in self.occupied_indices() {
            visit(voxel_index);
        }
    }
}

type BlockStorage = PaletteStorage<VoxelCell>;
type FluidStorage = PaletteStorage<FluidCell>;

fn shared_empty_blocks() -> Arc<BlockStorage> {
    static EMPTY_BLOCKS: OnceLock<Arc<BlockStorage>> = OnceLock::new();
    Arc::clone(EMPTY_BLOCKS.get_or_init(|| Arc::new(BlockStorage::default())))
}

fn shared_empty_fluids() -> Arc<FluidStorage> {
    static EMPTY_FLUIDS: OnceLock<Arc<FluidStorage>> = OnceLock::new();
    Arc::clone(EMPTY_FLUIDS.get_or_init(|| Arc::new(FluidStorage::default())))
}

fn shared_uniform_sky_light(sky: u8) -> Arc<[VoxelLight]> {
    static UNIFORM_LIGHT: OnceLock<[Arc<[VoxelLight]>; 16]> = OnceLock::new();
    let levels = UNIFORM_LIGHT.get_or_init(|| {
        std::array::from_fn(|level| {
            Arc::from(vec![
                VoxelLight::new_hsi(level as u8, BlockLight::DARK);
                CHUNK_VOLUME
            ])
        })
    });
    Arc::clone(&levels[usize::from(sky.min(VoxelLight::MAX_LEVEL))])
}

fn shared_dark_light() -> Arc<[VoxelLight]> {
    shared_uniform_sky_light(0)
}

fn shared_empty_fluid_bits() -> Arc<[u64; FLUID_FRONTIER_WORDS]> {
    static EMPTY_FLUID_BITS: OnceLock<Arc<[u64; FLUID_FRONTIER_WORDS]>> = OnceLock::new();
    Arc::clone(EMPTY_FLUID_BITS.get_or_init(|| Arc::new([0; FLUID_FRONTIER_WORDS])))
}

fn shared_empty_layers() -> Arc<HashMap<u16, Vec<AttachedLayer>>> {
    static EMPTY_LAYERS: OnceLock<Arc<HashMap<u16, Vec<AttachedLayer>>>> = OnceLock::new();
    Arc::clone(EMPTY_LAYERS.get_or_init(|| Arc::new(HashMap::new())))
}

fn shared_empty_objects() -> Arc<HashMap<u16, ObjectCell>> {
    static EMPTY_OBJECTS: OnceLock<Arc<HashMap<u16, ObjectCell>>> = OnceLock::new();
    Arc::clone(EMPTY_OBJECTS.get_or_init(|| Arc::new(HashMap::new())))
}

#[derive(Component, Clone)]
pub struct VoxelChunk {
    blocks: Arc<BlockStorage>,
    fluids: Arc<FluidStorage>,
    layers: Arc<HashMap<u16, Vec<AttachedLayer>>>,
    objects: Arc<HashMap<u16, ObjectCell>>,
    light: Arc<[VoxelLight]>,
    block_count: usize,
    fluid_count: usize,
    layer_count: usize,
    fluid_frontier_sources: Arc<[u64; FLUID_FRONTIER_WORDS]>,
    dynamic_fluid_cells: Arc<[u64; FLUID_FRONTIER_WORDS]>,
    boundary_content_counts: [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: [u16; BOUNDARY_FACE_COUNT],
}

pub(crate) struct VoxelChunkInitialBlocksMut<'a> {
    blocks: &'a mut BlockStorage,
    palette_indices: HashMap<VoxelCell, u16>,
    block_count: &'a mut usize,
    boundary_content_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
}

impl VoxelChunkInitialBlocksMut<'_> {
    pub(crate) fn set_block(&mut self, x: usize, y: usize, z: usize, block: VoxelCell) {
        let voxel_index = index(x, y, z);
        debug_assert!(
            self.blocks.get(voxel_index).is_none(),
            "initial chunk material pass cannot overwrite a block"
        );
        let palette_index = if let Some(&palette_index) = self.palette_indices.get(&block) {
            palette_index
        } else {
            self.blocks.palette.push(block);
            self.blocks.usage.push(0);
            let palette_index = u16::try_from(self.blocks.palette.len())
                .expect("initial block palette cannot exceed u16 index space");
            self.palette_indices.insert(block, palette_index);
            palette_index
        };
        self.blocks.set_palette_index(voxel_index, palette_index);
        let usage = &mut self.blocks.usage[palette_index as usize - 1];
        *usage = usage
            .checked_add(1)
            .expect("initial block palette usage cannot overflow");
        *self.block_count += 1;
        adjust_boundary_counts(self.boundary_content_counts, x, y, z, true);
    }
}

pub(crate) struct VoxelChunkStructureMut<'a> {
    blocks: &'a mut BlockStorage,
    fluids: &'a mut FluidStorage,
    layers: &'a mut HashMap<u16, Vec<AttachedLayer>>,
    objects: &'a mut HashMap<u16, ObjectCell>,
    block_palette_indices: HashMap<VoxelCell, u16>,
    block_count: &'a mut usize,
    fluid_count: &'a mut usize,
    layer_count: &'a mut usize,
    dynamic_fluid_cells: &'a mut [u64; FLUID_FRONTIER_WORDS],
    boundary_content_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
}

impl VoxelChunkStructureMut<'_> {
    pub(crate) fn cell_at(&self, x: usize, y: usize, z: usize) -> Option<VoxelCell> {
        self.blocks.get(index(x, y, z))
    }

    pub(crate) fn fluid_at(&self, x: usize, y: usize, z: usize) -> Option<FluidCell> {
        self.fluids.get(index(x, y, z))
    }

    pub(crate) fn set_block(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block: VoxelCell,
    ) {
        let voxel_index = index(x, y, z);
        let previous = self.blocks.get(voxel_index);
        if previous == Some(block) {
            return;
        }

        let had_block = previous.is_some();
        let had_content = had_block || self.fluids.get(voxel_index).is_some();

        if previous.map(|cell| cell.block_id) != Some(block.block_id) {
            if let Some(removed) = self.layers.remove(&(voxel_index as u16)) {
                *self.layer_count = self
                    .layer_count
                    .checked_sub(removed.len())
                    .expect("chunk layer count cannot underflow");
            }
            self.objects.remove(&(voxel_index as u16));
        }

        let next = if let Some(&palette_index) = self.block_palette_indices.get(&block) {
            palette_index
        } else {
            let reusable = self.blocks.usage.iter().position(|&usage| usage == 0);
            let palette_index = if let Some(index) = reusable {
                self.blocks.palette[index] = block;
                u16::try_from(index + 1)
                    .expect("structure block palette index must fit u16")
            } else {
                self.blocks.palette.push(block);
                self.blocks.usage.push(0);
                u16::try_from(self.blocks.palette.len())
                    .expect("structure block palette cannot exceed u16 index space")
            };
            self.block_palette_indices.insert(block, palette_index);
            palette_index
        };

        let previous_index = self.blocks.indices[voxel_index];
        if previous_index != 0 {
            let usage = &mut self.blocks.usage[previous_index as usize - 1];
            *usage = usage
                .checked_sub(1)
                .expect("structure block palette usage cannot underflow");
            if *usage == 0
                && let Some(previous) = previous
            {
                self.block_palette_indices.remove(&previous);
            }
        }

        self.blocks.set_palette_index(voxel_index, next);
        let usage = &mut self.blocks.usage[next as usize - 1];
        *usage = usage
            .checked_add(1)
            .expect("structure block palette usage cannot overflow");

        if !had_block {
            *self.block_count += 1;
        }
        if !had_content {
            adjust_boundary_counts(self.boundary_content_counts, x, y, z, true);
        }
    }

    pub(crate) fn clear_block(&mut self, x: usize, y: usize, z: usize) {
        let voxel_index = index(x, y, z);
        let previous_index = self.blocks.indices[voxel_index];
        if previous_index == 0 {
            return;
        }

        let previous = self.blocks.get(voxel_index);
        let became_unused = {
            let usage = &mut self.blocks.usage[previous_index as usize - 1];
            *usage = usage
                .checked_sub(1)
                .expect("structure block palette usage cannot underflow");
            *usage == 0
        };
        self.blocks.set_palette_index(voxel_index, 0);
        *self.block_count = self
            .block_count
            .checked_sub(1)
            .expect("chunk block count cannot underflow");
        if became_unused
            && let Some(previous) = previous
        {
            self.block_palette_indices.remove(&previous);
        }
        if let Some(removed) = self.layers.remove(&(voxel_index as u16)) {
            *self.layer_count = self
                .layer_count
                .checked_sub(removed.len())
                .expect("chunk layer count cannot underflow");
        }
        self.objects.remove(&(voxel_index as u16));
        if self.fluids.get(voxel_index).is_none() {
            adjust_boundary_counts(self.boundary_content_counts, x, y, z, false);
        }
    }

    pub(crate) fn add_layer(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        face: LayerFace,
        layer: LayerCell,
    ) -> bool {
        add_layer_in_storage(
            self.blocks,
            self.layers,
            self.layer_count,
            x,
            y,
            z,
            face,
            layer,
        )
    }

    pub(crate) fn set_object(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        object: ObjectCell,
    ) -> bool {
        set_object_in_storage(self.blocks, self.objects, x, y, z, object)
    }

    pub(crate) fn clear_fluid(&mut self, x: usize, y: usize, z: usize) {
        let voxel_index = index(x, y, z);
        let previous_index = self.fluids.indices[voxel_index];
        if previous_index == 0 {
            return;
        }

        let usage = &mut self.fluids.usage[previous_index as usize - 1];
        *usage = usage
            .checked_sub(1)
            .expect("structure fluid palette usage cannot underflow");
        self.fluids.set_palette_index(voxel_index, 0);
        *self.fluid_count = self
            .fluid_count
            .checked_sub(1)
            .expect("chunk fluid count cannot underflow");
        adjust_boundary_counts(self.boundary_fluid_counts, x, y, z, false);
        if self.blocks.get(voxel_index).is_none() {
            adjust_boundary_counts(self.boundary_content_counts, x, y, z, false);
        }
        set_voxel_bit(self.dynamic_fluid_cells, voxel_index, false);
    }

    pub(crate) fn set_fluid(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        fluid: FluidCell,
    ) {
        let voxel_index = index(x, y, z);
        let previous = self.fluids.get(voxel_index);
        if previous == Some(fluid) {
            return;
        }

        let had_fluid = previous.is_some();
        let had_content = had_fluid || self.blocks.get(voxel_index).is_some();
        self.fluids.set(voxel_index, Some(fluid));
        if !had_fluid {
            *self.fluid_count += 1;
            adjust_boundary_counts(self.boundary_fluid_counts, x, y, z, true);
            if !had_content {
                adjust_boundary_counts(self.boundary_content_counts, x, y, z, true);
            }
        }
        set_voxel_bit(
            self.dynamic_fluid_cells,
            voxel_index,
            !fluid.is_source(),
        );
    }
}

pub(crate) struct VoxelChunkInitialFluidsMut<'a> {
    blocks: &'a BlockStorage,
    fluids: &'a mut FluidStorage,
    palette_indices: HashMap<FluidCell, u16>,
    fluid_count: &'a mut usize,
    dynamic_fluid_cells: &'a mut [u64; FLUID_FRONTIER_WORDS],
    boundary_content_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
}

impl VoxelChunkInitialFluidsMut<'_> {
    pub(crate) fn set_fluid(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        fluid: FluidCell,
    ) {
        let voxel_index = index(x, y, z);
        debug_assert!(
            self.fluids.get(voxel_index).is_none(),
            "initial chunk fluid pass cannot overwrite fluid"
        );

        let palette_index = if let Some(&palette_index) = self.palette_indices.get(&fluid) {
            palette_index
        } else {
            self.fluids.palette.push(fluid);
            self.fluids.usage.push(0);
            let palette_index = u16::try_from(self.fluids.palette.len())
                .expect("initial fluid palette cannot exceed u16 index space");
            self.palette_indices.insert(fluid, palette_index);
            palette_index
        };
        self.fluids.set_palette_index(voxel_index, palette_index);
        let usage = &mut self.fluids.usage[palette_index as usize - 1];
        *usage = usage
            .checked_add(1)
            .expect("initial fluid palette usage cannot overflow");

        *self.fluid_count += 1;
        adjust_boundary_counts(self.boundary_fluid_counts, x, y, z, true);
        if self.blocks.get(voxel_index).is_none() {
            adjust_boundary_counts(self.boundary_content_counts, x, y, z, true);
        }
        set_voxel_bit(
            self.dynamic_fluid_cells,
            voxel_index,
            !fluid.is_source(),
        );
    }
}

pub(crate) struct VoxelChunkFluidsMut<'a> {
    blocks: &'a BlockStorage,
    fluids: &'a mut FluidStorage,
    fluid_count: &'a mut usize,
    fluid_frontier_sources: &'a mut [u64; FLUID_FRONTIER_WORDS],
    dynamic_fluid_cells: &'a mut [u64; FLUID_FRONTIER_WORDS],
    boundary_content_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: &'a mut [u16; BOUNDARY_FACE_COUNT],
}

impl VoxelChunkFluidsMut<'_> {
    pub(crate) fn set_fluid(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        fluid: Option<FluidCell>,
    ) {
        set_fluid_in_storage(
            self.blocks,
            &mut *self.fluids,
            &mut *self.fluid_count,
            &mut *self.fluid_frontier_sources,
            &mut *self.dynamic_fluid_cells,
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
            layers: shared_empty_layers(),
            objects: shared_empty_objects(),
            light: shared_dark_light(),
            block_count: 0,
            fluid_count: 0,
            layer_count: 0,
            fluid_frontier_sources: shared_empty_fluid_bits(),
            dynamic_fluid_cells: shared_empty_fluid_bits(),
            boundary_content_counts: [0; BOUNDARY_FACE_COUNT],
            boundary_fluid_counts: [0; BOUNDARY_FACE_COUNT],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.block_count == 0
            && self.fluid_count == 0
            && self.layer_count == 0
            && self.objects.is_empty()
    }

    pub(crate) fn has_fluid(&self) -> bool {
        self.fluid_count > 0
    }

    pub(crate) fn has_fluid_settling_work(&self) -> bool {
        self.dynamic_fluid_cells.iter().any(|word| *word != 0)
            || self.fluid_frontier_sources.iter().any(|word| *word != 0)
    }

    pub(crate) fn suppress_generated_fluid_frontiers(&mut self, positions: &[[u8; 3]]) {
        if positions.is_empty() {
            return;
        }

        let sources = Arc::make_mut(&mut self.fluid_frontier_sources);
        for [x, y, z] in positions {
            set_voxel_bit(
                sources,
                index(usize::from(*x), usize::from(*y), usize::from(*z)),
                false,
            );
        }
    }

    pub(crate) fn block_count(&self) -> usize {
        self.block_count
    }

    pub(crate) fn fluid_count(&self) -> usize {
        self.fluid_count
    }

    pub(crate) fn has_terrain_content(&self) -> bool {
        self.block_count > 0 || self.layer_count > 0
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
                let fluid = self.fluids.get(voxel_index)
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
        for (word_index, &word) in self.dynamic_fluid_cells.iter().enumerate() {
            let mut remaining = word;
            while remaining != 0 {
                let bit = remaining.trailing_zeros() as usize;
                let voxel_index = word_index * u64::BITS as usize + bit;
                if voxel_index >= CHUNK_VOLUME {
                    break;
                }
                let fluid = self.fluids.get(voxel_index)
                    .expect("dynamic fluid metadata must point to a fluid voxel");
                debug_assert!(!fluid.is_source());
                let (x, y, z) = coordinates(voxel_index);
                visit(IVec3::new(x as i32, y as i32, z as i32), fluid);
                remaining &= remaining - 1;
            }
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

    pub(crate) fn dependency_boundary_has_content(&self, outward: IVec3) -> bool {
        if boundary_face_index(outward).is_some() {
            return self.boundary_has_content(outward);
        }

        dependency_boundary_region_has(outward, |x, y, z| {
            let voxel_index = index(x, y, z);
            self.blocks.get_ref(voxel_index).is_some() || self.fluids.get(voxel_index).is_some()
        })
    }

    pub(crate) fn dependency_boundary_has_fluid(&self, outward: IVec3) -> bool {
        if boundary_face_index(outward).is_some() {
            return self.boundary_has_fluid(outward);
        }

        dependency_boundary_region_has(outward, |x, y, z| {
            self.fluids.get(index(x, y, z)).is_some()
        })
    }

    pub fn cell_at(&self, x: i32, y: i32, z: i32) -> Option<VoxelCell> {
        self.cell_ref_at(x, y, z).copied()
    }

    pub(crate) fn cell_at_local(&self, x: usize, y: usize, z: usize) -> Option<VoxelCell> {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        self.blocks.get(index(x, y, z))
    }

    pub(crate) fn cell_ref_at(&self, x: i32, y: i32, z: i32) -> Option<&VoxelCell> {
        if !in_bounds(x, y, z) {
            return None;
        }

        self.blocks
            .get_ref(index(x as usize, y as usize, z as usize))
    }

    pub(crate) fn occupied_block_voxels(
        &self,
    ) -> impl Iterator<Item = (usize, usize, usize, &VoxelCell)> + '_ {
        self.blocks.occupied_indices().map(|voxel_index| {
            let cell = self
                .blocks
                .get_ref(voxel_index)
                .expect("block occupancy metadata must point to a block voxel");
            let (x, y, z) = coordinates(voxel_index);
            (x, y, z, cell)
        })
    }

    pub(crate) fn visit_block_voxels<'a>(
        &'a self,
        mut visit: impl FnMut(usize, usize, usize, &'a VoxelCell),
    ) {
        for (x, y, z, cell) in self.occupied_block_voxels() {
            visit(x, y, z, cell);
        }
    }

    pub(crate) fn layers_at(&self, x: i32, y: i32, z: i32) -> &[AttachedLayer] {
        if !in_bounds(x, y, z) {
            return &[];
        }

        let index = index(x as usize, y as usize, z as usize) as u16;
        self.layers.get(&index).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn layer_groups(
        &self,
    ) -> impl Iterator<Item = (usize, &[AttachedLayer])> + '_ {
        self.layers
            .iter()
            .map(|(&index, layers)| (index as usize, layers.as_slice()))
    }

    pub(crate) fn object_at(&self, x: i32, y: i32, z: i32) -> Option<ObjectCell> {
        if !in_bounds(x, y, z) {
            return None;
        }
        self.objects.get(&(index(x as usize, y as usize, z as usize) as u16)).copied()
    }

    pub(crate) fn object_entries(&self) -> impl Iterator<Item = (usize, ObjectCell)> + '_ {
        self.objects
            .iter()
            .map(|(&index, &object)| (index as usize, object))
    }

    pub(crate) fn object_voxels(
        &self,
    ) -> impl Iterator<Item = (usize, usize, usize, ObjectCell)> + '_ {
        self.object_entries().map(|(voxel_index, object)| {
            let (x, y, z) = coordinates(voxel_index);
            (x, y, z, object)
        })
    }

    pub fn fluid_at(&self, x: i32, y: i32, z: i32) -> Option<FluidCell> {
        if !in_bounds(x, y, z) {
            return None;
        }

        self.fluid_at_local(x as usize, y as usize, z as usize)
    }

    pub(crate) fn fluid_at_local(&self, x: usize, y: usize, z: usize) -> Option<FluidCell> {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        self.fluids.get(index(x, y, z))
    }

    pub(crate) fn content_at_local(
        &self,
        x: usize,
        y: usize,
        z: usize,
    ) -> (Option<VoxelCell>, Option<FluidCell>) {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        let voxel_index = index(x, y, z);
        (self.blocks.get(voxel_index), self.fluids.get(voxel_index))
    }

    pub(crate) fn visit_content_voxels(
        &self,
        mut visit: impl FnMut(
            usize,
            usize,
            usize,
            Option<VoxelCell>,
            Option<FluidCell>,
        ),
    ) {
        for (word_index, (&block_word, &fluid_word)) in self
            .blocks
            .occupied
            .iter()
            .zip(self.fluids.occupied.iter())
            .enumerate()
        {
            let mut remaining = block_word | fluid_word;
            while remaining != 0 {
                let bit = remaining.trailing_zeros() as usize;
                let voxel_index = word_index * u64::BITS as usize + bit;
                if voxel_index >= CHUNK_VOLUME {
                    break;
                }

                let (x, y, z) = coordinates(voxel_index);
                visit(
                    x,
                    y,
                    z,
                    self.blocks.get(voxel_index),
                    self.fluids.get(voxel_index),
                );
                remaining &= remaining - 1;
            }
        }
    }

    pub(crate) fn visit_fluid_voxels(
        &self,
        mut visit: impl FnMut(usize, usize, usize, FluidCell),
    ) {
        self.fluids.visit_occupied_indices(|voxel_index| {
            let fluid = self
                .fluids
                .get(voxel_index)
                .expect("fluid occupancy metadata must point to a fluid voxel");
            let (x, y, z) = coordinates(voxel_index);
            visit(x, y, z, fluid);
        });
    }

    pub(crate) fn light_at(&self, x: i32, y: i32, z: i32) -> VoxelLight {
        if !in_bounds(x, y, z) {
            return VoxelLight::DARK;
        }

        self.light_at_local(x as usize, y as usize, z as usize)
    }

    pub(crate) fn light_at_local(&self, x: usize, y: usize, z: usize) -> VoxelLight {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        self.light[index(x, y, z)]
    }

    pub(crate) fn sample_local_at(
        &self,
        x: usize,
        y: usize,
        z: usize,
    ) -> (Option<VoxelCell>, Option<FluidCell>, VoxelLight) {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        let voxel_index = index(x, y, z);
        (
            self.blocks.get(voxel_index),
            self.fluids.get(voxel_index),
            self.light[voxel_index],
        )
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

        Some(self.sample_local_at(x as usize, y as usize, z as usize))
    }

    pub(crate) fn edit_initial_blocks<R>(
        &mut self,
        edit: impl FnOnce(&mut VoxelChunkInitialBlocksMut<'_>) -> R,
    ) -> R {
        debug_assert_eq!(self.block_count, 0);
        debug_assert_eq!(self.fluid_count, 0);
        debug_assert_eq!(self.layer_count, 0);

        let blocks = Arc::make_mut(&mut self.blocks);
        let mut content = VoxelChunkInitialBlocksMut {
            blocks,
            palette_indices: HashMap::new(),
            block_count: &mut self.block_count,
            boundary_content_counts: &mut self.boundary_content_counts,
        };
        edit(&mut content)
    }

    pub(crate) fn edit_structure_content<R>(
        &mut self,
        edit: impl FnOnce(&mut VoxelChunkStructureMut<'_>) -> R,
    ) -> R {
        let blocks = Arc::make_mut(&mut self.blocks);
        let fluids = Arc::make_mut(&mut self.fluids);
        let layers = Arc::make_mut(&mut self.layers);
        let objects = Arc::make_mut(&mut self.objects);
        let dynamic_fluid_cells = Arc::make_mut(&mut self.dynamic_fluid_cells);
        let block_palette_indices = blocks
            .palette
            .iter()
            .copied()
            .zip(blocks.usage.iter().copied())
            .enumerate()
            .filter_map(|(index, (cell, usage))| {
                (usage > 0).then_some((
                    cell,
                    u16::try_from(index + 1)
                        .expect("active block palette index must fit u16"),
                ))
            })
            .collect();

        let result = {
            let mut content = VoxelChunkStructureMut {
                blocks,
                fluids,
                layers,
                objects,
                block_palette_indices,
                block_count: &mut self.block_count,
                fluid_count: &mut self.fluid_count,
                layer_count: &mut self.layer_count,
                dynamic_fluid_cells,
                boundary_content_counts: &mut self.boundary_content_counts,
                boundary_fluid_counts: &mut self.boundary_fluid_counts,
            };
            edit(&mut content)
        };

        let sources = Arc::make_mut(&mut self.fluid_frontier_sources);
        sources.fill(0);
        if self.fluid_count > 0 {
            rebuild_fluid_frontier_sources(
                self.blocks.as_ref(),
                self.fluids.as_ref(),
                sources,
            );
        }
        result
    }

    pub(crate) fn edit_initial_fluids<R>(
        &mut self,
        edit: impl FnOnce(&mut VoxelChunkInitialFluidsMut<'_>) -> R,
    ) -> R {
        debug_assert_eq!(self.fluid_count, 0);

        let fluids = Arc::make_mut(&mut self.fluids);
        let dynamic_fluid_cells = Arc::make_mut(&mut self.dynamic_fluid_cells);
        let result = {
            let mut content = VoxelChunkInitialFluidsMut {
                blocks: self.blocks.as_ref(),
                fluids,
                palette_indices: HashMap::new(),
                fluid_count: &mut self.fluid_count,
                dynamic_fluid_cells,
                boundary_content_counts: &mut self.boundary_content_counts,
                boundary_fluid_counts: &mut self.boundary_fluid_counts,
            };
            edit(&mut content)
        };

        let fluid_frontier_sources = Arc::make_mut(&mut self.fluid_frontier_sources);
        fluid_frontier_sources.fill(0);
        rebuild_fluid_frontier_sources(
            self.blocks.as_ref(),
            self.fluids.as_ref(),
            fluid_frontier_sources,
        );
        result
    }

    pub(crate) fn edit_fluids<R>(
        &mut self,
        edit: impl FnOnce(&mut VoxelChunkFluidsMut<'_>) -> R,
    ) -> R {
        let fluids = Arc::make_mut(&mut self.fluids);
        let fluid_frontier_sources = Arc::make_mut(&mut self.fluid_frontier_sources);
        let dynamic_fluid_cells = Arc::make_mut(&mut self.dynamic_fluid_cells);
        let mut content = VoxelChunkFluidsMut {
            blocks: self.blocks.as_ref(),
            fluids,
            fluid_count: &mut self.fluid_count,
            fluid_frontier_sources,
            dynamic_fluid_cells,
            boundary_content_counts: &mut self.boundary_content_counts,
            boundary_fluid_counts: &mut self.boundary_fluid_counts,
        };
        edit(&mut content)
    }

    #[cfg(test)]
    pub(crate) fn set_block(&mut self, x: usize, y: usize, z: usize, block: Option<VoxelCell>) {
        let _ = self.set_block_with_detached_object(x, y, z, block);
    }

    pub(crate) fn set_block_with_detached_object(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block: Option<VoxelCell>,
    ) -> Option<ObjectCell> {
        let blocks = Arc::make_mut(&mut self.blocks);
        let layers = Arc::make_mut(&mut self.layers);
        let objects = Arc::make_mut(&mut self.objects);
        let fluid_frontier_sources = Arc::make_mut(&mut self.fluid_frontier_sources);
        set_block_in_storage(
            blocks,
            self.fluids.as_ref(),
            layers,
            objects,
            &mut self.block_count,
            &mut self.layer_count,
            fluid_frontier_sources,
            &mut self.boundary_content_counts,
            x,
            y,
            z,
            block,
        )
    }

    pub(crate) fn add_layer(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        face: LayerFace,
        layer: LayerCell,
    ) -> bool {
        let layers = Arc::make_mut(&mut self.layers);
        add_layer_in_storage(
            self.blocks.as_ref(),
            layers,
            &mut self.layer_count,
            x,
            y,
            z,
            face,
            layer,
        )
    }

    pub(crate) fn remove_layer(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        face: LayerFace,
        layer_id: &str,
    ) -> bool {
        let layers = Arc::make_mut(&mut self.layers);
        remove_layer_in_storage(layers, &mut self.layer_count, x, y, z, face, layer_id)
    }

    pub(crate) fn set_object(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        object: ObjectCell,
    ) -> bool {
        let objects = Arc::make_mut(&mut self.objects);
        set_object_in_storage(self.blocks.as_ref(), objects, x, y, z, object)
    }

    pub(crate) fn remove_object(&mut self, x: usize, y: usize, z: usize) -> Option<ObjectCell> {
        Arc::make_mut(&mut self.objects).remove(&(index(x, y, z) as u16))
    }

    pub(crate) fn set_fluid(&mut self, x: usize, y: usize, z: usize, fluid: Option<FluidCell>) {
        let fluids = Arc::make_mut(&mut self.fluids);
        let fluid_frontier_sources = Arc::make_mut(&mut self.fluid_frontier_sources);
        let dynamic_fluid_cells = Arc::make_mut(&mut self.dynamic_fluid_cells);
        set_fluid_in_storage(
            self.blocks.as_ref(),
            fluids,
            &mut self.fluid_count,
            fluid_frontier_sources,
            dynamic_fluid_cells,
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
                    lights[index] = light_at(x, y, z, blocks.get(index), fluids.get(index));
                }
            }
        }
    }

    pub(crate) fn rebuild_empty_light_columns(&mut self, sky_by_column: &[u8; CHUNK_AREA]) {
        debug_assert!(self.is_empty(), "empty light rebuild requires an empty chunk");

        if let Some(&sky) = sky_by_column.first()
            && sky_by_column.iter().all(|&candidate| candidate == sky)
        {
            self.light = shared_uniform_sky_light(sky);
            return;
        }

        let lights = Arc::make_mut(&mut self.light);
        for (light, &sky) in lights[..CHUNK_AREA].iter_mut().zip(sky_by_column) {
            *light = VoxelLight::new_hsi(sky, BlockLight::DARK);
        }
        for y in 1..CHUNK_SIZE {
            lights.copy_within(0..CHUNK_AREA, y * CHUNK_AREA);
        }
    }

    #[cfg(test)]
    pub(crate) fn clear_light(&mut self) {
        self.light = shared_dark_light();
    }

}

#[expect(
    clippy::too_many_arguments,
    reason = "block mutation keeps chunk occupancy metadata updates atomic"
)]
fn set_block_in_storage(
    blocks: &mut BlockStorage,
    fluids: &FluidStorage,
    layers: &mut HashMap<u16, Vec<AttachedLayer>>,
    objects: &mut HashMap<u16, ObjectCell>,
    block_count: &mut usize,
    layer_count: &mut usize,
    fluid_frontier_sources: &mut [u64; FLUID_FRONTIER_WORDS],
    boundary_content_counts: &mut [u16; BOUNDARY_FACE_COUNT],
    x: usize,
    y: usize,
    z: usize,
    block: Option<VoxelCell>,
) -> Option<ObjectCell> {
    let index = index(x, y, z);
    let previous_block = blocks.get(index);
    let had_block = previous_block.is_some();
    let had_content = had_block || fluids.get(index).is_some();
    let has_block = block.is_some();

    let detached_object =
        if previous_block.map(|cell| cell.block_id) != block.map(|cell| cell.block_id) {
            if let Some(removed) = layers.remove(&(index as u16)) {
                *layer_count = layer_count
                    .checked_sub(removed.len())
                    .expect("chunk layer count cannot underflow");
            }
            objects.remove(&(index as u16))
        } else {
            None
        };

    let has_content = has_block || fluids.get(index).is_some();

    if had_block != has_block {
        adjust_total_count(block_count, has_block);
    }
    if had_content != has_content {
        adjust_boundary_counts(boundary_content_counts, x, y, z, has_content);
    }

    blocks.set(index, block);
    refresh_fluid_frontier_sources_near(blocks, fluids, fluid_frontier_sources, x, y, z);
    detached_object
}

#[expect(
    clippy::too_many_arguments,
    reason = "layer mutation keeps support, identity and sparse storage updates atomic"
)]
fn add_layer_in_storage(
    blocks: &BlockStorage,
    layers: &mut HashMap<u16, Vec<AttachedLayer>>,
    layer_count: &mut usize,
    x: usize,
    y: usize,
    z: usize,
    face: LayerFace,
    layer: LayerCell,
) -> bool {
    let index = index(x, y, z);
    if blocks.get(index).is_none() {
        return false;
    }

    let entries = layers.entry(index as u16).or_default();
    if let Some(existing) = entries
        .iter_mut()
        .find(|existing| existing.face == face && existing.cell.layer_id == layer.layer_id)
    {
        if existing.cell == layer {
            return false;
        }
        existing.cell = layer;
        return true;
    }

    assert!(
        entries.len() < MAX_LAYERS_PER_VOXEL,
        "voxel cannot contain more than {MAX_LAYERS_PER_VOXEL} layers"
    );
    entries.push(AttachedLayer { face, cell: layer });
    *layer_count += 1;
    true
}

fn set_object_in_storage(
    blocks: &BlockStorage,
    objects: &mut HashMap<u16, ObjectCell>,
    x: usize,
    y: usize,
    z: usize,
    object: ObjectCell,
) -> bool {
    let voxel_index = index(x, y, z);
    if blocks.get(voxel_index).is_none() {
        return false;
    }
    let key = voxel_index as u16;
    if objects.get(&key).copied() == Some(object) {
        return false;
    }
    objects.insert(key, object);
    true
}

fn remove_layer_in_storage(
    layers: &mut HashMap<u16, Vec<AttachedLayer>>,
    layer_count: &mut usize,
    x: usize,
    y: usize,
    z: usize,
    face: LayerFace,
    layer_id: &str,
) -> bool {
    let key = index(x, y, z) as u16;
    let Some(entries) = layers.get_mut(&key) else {
        return false;
    };
    let Some(position) = entries
        .iter()
        .position(|entry| entry.face == face && entry.cell.layer_id == layer_id)
    else {
        return false;
    };

    entries.remove(position);
    *layer_count = layer_count
        .checked_sub(1)
        .expect("chunk layer count cannot underflow");
    if entries.is_empty() {
        layers.remove(&key);
    }
    true
}

#[expect(
    clippy::too_many_arguments,
    reason = "fluid mutation keeps occupancy and boundary metadata updates atomic"
)]
fn set_fluid_in_storage(
    blocks: &BlockStorage,
    fluids: &mut FluidStorage,
    fluid_count: &mut usize,
    fluid_frontier_sources: &mut [u64; FLUID_FRONTIER_WORDS],
    dynamic_fluid_cells: &mut [u64; FLUID_FRONTIER_WORDS],
    boundary_content_counts: &mut [u16; BOUNDARY_FACE_COUNT],
    boundary_fluid_counts: &mut [u16; BOUNDARY_FACE_COUNT],
    x: usize,
    y: usize,
    z: usize,
    fluid: Option<FluidCell>,
) {
    let index = index(x, y, z);
    let previous_fluid = fluids.get(index);
    let had_fluid = previous_fluid.is_some();
    let had_content = had_fluid || blocks.get(index).is_some();
    let has_fluid = fluid.is_some();
    let has_content = has_fluid || blocks.get(index).is_some();

    if had_fluid != has_fluid {
        adjust_total_count(fluid_count, has_fluid);
        adjust_boundary_counts(boundary_fluid_counts, x, y, z, has_fluid);
    }
    if had_content != has_content {
        adjust_boundary_counts(boundary_content_counts, x, y, z, has_content);
    }

    fluids.set(index, fluid);
    set_voxel_bit(
        dynamic_fluid_cells,
        index,
        fluid.is_some_and(|fluid| !fluid.is_source()),
    );
    refresh_fluid_frontier_sources_near(blocks, fluids, fluid_frontier_sources, x, y, z);
}

fn set_voxel_bit(bits: &mut [u64; FLUID_FRONTIER_WORDS], voxel_index: usize, value: bool) {
    let word = voxel_index / u64::BITS as usize;
    let mask = 1_u64 << (voxel_index % u64::BITS as usize);
    if value {
        bits[word] |= mask;
    } else {
        bits[word] &= !mask;
    }
}

fn rebuild_fluid_frontier_sources(
    blocks: &BlockStorage,
    fluids: &FluidStorage,
    sources: &mut [u64; FLUID_FRONTIER_WORDS],
) {
    for voxel_index in fluids.occupied_indices() {
        let (x, y, z) = coordinates(voxel_index);
        refresh_fluid_frontier_source(
            blocks,
            fluids,
            sources,
            IVec3::new(x as i32, y as i32, z as i32),
        );
    }
}

fn refresh_fluid_frontier_sources_near(
    blocks: &BlockStorage,
    fluids: &FluidStorage,
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
    blocks: &BlockStorage,
    fluids: &FluidStorage,
    sources: &mut [u64; FLUID_FRONTIER_WORDS],
    source: IVec3,
) {
    let source_index = index(source.x as usize, source.y as usize, source.z as usize);
    let should_track = fluids.get(source_index).is_some()
        && FLUID_SPREAD_TARGETS.iter().any(|offset| {
            let target = source + *offset;
            if !in_bounds(target.x, target.y, target.z) {
                return true;
            }
            let target_index = index(target.x as usize, target.y as usize, target.z as usize);
            blocks.get(target_index).is_none() && fluids.get(target_index).is_none()
        });

    set_voxel_bit(sources, source_index, should_track);
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

fn dependency_boundary_region_has(
    outward: IVec3,
    mut has_content: impl FnMut(usize, usize, usize) -> bool,
) -> bool {
    debug_assert_ne!(outward, IVec3::ZERO);
    debug_assert!(
        [-1, 0, 1].contains(&outward.x)
            && [-1, 0, 1].contains(&outward.y)
            && [-1, 0, 1].contains(&outward.z)
    );

    let axis_range = |component: i32| match component {
        -1 => (0, 1),
        0 => (0, CHUNK_SIZE),
        1 => (CHUNK_SIZE - 1, CHUNK_SIZE),
        _ => unreachable!("dependency boundary offset must stay in -1..=1"),
    };
    let (x_start, x_end) = axis_range(outward.x);
    let (y_start, y_end) = axis_range(outward.y);
    let (z_start, z_end) = axis_range(outward.z);

    for y in y_start..y_end {
        for z in z_start..z_end {
            for x in x_start..x_end {
                if has_content(x, y, z) {
                    return true;
                }
            }
        }
    }

    false
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
    fn empty_chunks_share_all_zeroed_storage() {
        let first = VoxelChunk::empty();
        let second = VoxelChunk::empty();

        assert!(Arc::ptr_eq(&first.blocks, &second.blocks));
        assert!(Arc::ptr_eq(&first.fluids, &second.fluids));
        assert!(Arc::ptr_eq(&first.layers, &second.layers));
        assert!(Arc::ptr_eq(&first.light, &second.light));
        assert!(Arc::ptr_eq(
            &first.fluid_frontier_sources,
            &second.fluid_frontier_sources
        ));
        assert!(Arc::ptr_eq(
            &first.dynamic_fluid_cells,
            &second.dynamic_fluid_cells
        ));
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
    fn layers_stack_per_face_and_follow_support_identity() {
        let mut chunk = VoxelChunk::empty();
        let support = VoxelCell::new("stone", Default::default());
        chunk.set_block(2, 3, 4, Some(support));

        let moss = LayerCell::new("asteria:moss", Default::default());
        let lichen = LayerCell::new("asteria:lichen", Default::default());
        assert!(chunk.add_layer(2, 3, 4, LayerFace::Top, moss));
        assert!(chunk.add_layer(2, 3, 4, LayerFace::Top, lichen));
        assert_eq!(chunk.layers_at(2, 3, 4).len(), 2);

        chunk.set_block(
            2,
            3,
            4,
            Some(VoxelCell::new("dirt", Default::default())),
        );
        assert!(chunk.layers_at(2, 3, 4).is_empty());
    }

    #[test]
    fn layers_require_a_supporting_block() {
        let mut chunk = VoxelChunk::empty();
        assert!(!chunk.add_layer(
            2,
            3,
            4,
            LayerFace::Top,
            LayerCell::new("asteria:moss", Default::default()),
        ));
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
    fn local_content_accessors_match_checked_access() {
        let mut chunk = VoxelChunk::empty();
        let block = VoxelCell::new("stone", Default::default());
        let fluid = FluidCell::source(0, 8);

        chunk.set_block(3, 5, 7, Some(block));
        chunk.set_fluid(3, 5, 7, Some(fluid));

        assert_eq!(chunk.cell_at_local(3, 5, 7), chunk.cell_at(3, 5, 7));
        assert_eq!(chunk.fluid_at_local(3, 5, 7), chunk.fluid_at(3, 5, 7));
        assert_eq!(
            chunk.content_at_local(3, 5, 7),
            (Some(block), Some(fluid)),
        );
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
        assert_eq!(
            chunk.sample_local_at(1, 2, 3),
            chunk.sample_local(1, 2, 3).unwrap(),
        );
        assert_eq!(
            chunk.sample_local_at(4, 5, 6),
            chunk.sample_local(4, 5, 6).unwrap(),
        );
        assert_eq!(
            chunk.sample_local_at(7, 8, 9),
            chunk.sample_local(7, 8, 9).unwrap(),
        );
        assert_eq!(chunk.light_at_local(7, 8, 9), chunk.light_at(7, 8, 9));
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
    fn independent_content_edits_preserve_metadata() {
        let mut chunk = VoxelChunk::empty();
        let last = CHUNK_SIZE - 1;
        let block = VoxelCell::new("stone", Default::default());
        let fluid = FluidCell::spreading(0, 7, 1);

        chunk.set_block(0, 0, 0, Some(block));
        chunk.set_fluid(last, last, last, Some(fluid));

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
