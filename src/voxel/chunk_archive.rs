use crate::content::{block_orientation::BlockOrientation, fluid::FluidId};

use super::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, CHUNK_VOLUME, VoxelChunk},
    fluid::FluidCell,
    secondary_properties::SecondaryProperties,
    texture_rotation::TextureRotation,
};

const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;
const OCCUPANCY_WORDS: usize = CHUNK_VOLUME.div_ceil(u64::BITS as usize);

#[derive(Clone, Copy)]
struct ArchivedCell {
    palette_index: u16,
    rotation: u8,
    orientation: u8,
    secondary_properties: SecondaryProperties,
}

#[derive(Clone, Copy)]
struct ArchivedFluidCell {
    fluid_id: FluidId,
    level: u8,
    source: bool,
    spread_distance: u16,
}

pub struct ArchivedChunk {
    occupancy: [u64; OCCUPANCY_WORDS],
    palette: Vec<&'static str>,
    cells: Vec<ArchivedCell>,
    fluid_occupancy: [u64; OCCUPANCY_WORDS],
    fluid_cells: Vec<ArchivedFluidCell>,
}

impl ArchivedChunk {
    pub fn from_chunk(chunk: &VoxelChunk) -> Self {
        let mut occupancy = [0_u64; OCCUPANCY_WORDS];
        let mut palette = Vec::<&'static str>::new();
        let mut cells = Vec::new();
        let mut fluid_occupancy = [0_u64; OCCUPANCY_WORDS];
        let mut fluid_cells = Vec::new();

        for index in 0..CHUNK_VOLUME {
            let (x, y, z) = coordinates(index);
            let (cell, fluid, _) = chunk
                .sample_local(x as i32, y as i32, z as i32)
                .expect("archive coordinates must stay inside the chunk");

            if let Some(cell) = cell {
                let palette_index = palette
                    .iter()
                    .position(|block_id| *block_id == cell.block_id)
                    .unwrap_or_else(|| {
                        palette.push(cell.block_id);
                        palette.len() - 1
                    });

                assert!(
                    palette_index <= u16::MAX as usize,
                    "chunk block palette cannot exceed {} entries",
                    u16::MAX
                );

                occupancy[index / u64::BITS as usize] |= 1_u64 << (index % u64::BITS as usize);
                cells.push(ArchivedCell {
                    palette_index: palette_index as u16,
                    rotation: rotation_index(cell.texture_rotation),
                    orientation: cell.orientation.index(),
                    secondary_properties: cell.secondary_properties(),
                });
            }

            if let Some(fluid) = fluid {
                fluid_occupancy[index / u64::BITS as usize] |=
                    1_u64 << (index % u64::BITS as usize);
                fluid_cells.push(ArchivedFluidCell {
                    fluid_id: fluid.fluid_id,
                    level: fluid.level,
                    source: fluid.is_source(),
                    spread_distance: fluid.spread_distance(),
                });
            }
        }

        Self {
            occupancy,
            palette,
            cells,
            fluid_occupancy,
            fluid_cells,
        }
    }

    pub(crate) fn block_entries(&self) -> impl Iterator<Item = (usize, VoxelCell)> + '_ {
        occupied_indices(&self.occupancy)
            .zip(self.cells.iter())
            .map(|(index, archived)| {
                let block_id = self.palette[archived.palette_index as usize];
                let cell = VoxelCell::oriented(
                    block_id,
                    TextureRotation::from_quarter_turn(archived.rotation),
                    BlockOrientation::from_index(archived.orientation),
                )
                .with_secondary_properties(archived.secondary_properties);
                (index, cell)
            })
    }

    pub(crate) fn fluid_entries(&self) -> impl Iterator<Item = (usize, FluidCell)> + '_ {
        occupied_indices(&self.fluid_occupancy)
            .zip(self.fluid_cells.iter())
            .map(|(index, archived)| {
                (
                    index,
                    FluidCell::with_state(
                        archived.fluid_id,
                        archived.level,
                        archived.source,
                        archived.spread_distance,
                    ),
                )
            })
    }

    pub fn restore(&self) -> VoxelChunk {
        let mut chunk = VoxelChunk::empty();

        chunk.edit_content(|content| {
            for (index, cell) in self.block_entries() {
                let (x, y, z) = coordinates(index);
                content.set_block(x, y, z, Some(cell));
            }
            for (index, fluid) in self.fluid_entries() {
                let (x, y, z) = coordinates(index);
                content.set_fluid(x, y, z, Some(fluid));
            }
        });

        chunk
    }
}

fn occupied_indices(
    occupancy: &[u64; OCCUPANCY_WORDS],
) -> impl Iterator<Item = usize> + '_ {
    occupancy.iter().enumerate().flat_map(|(word_index, &word)| {
        let mut remaining = word;
        std::iter::from_fn(move || {
            if remaining == 0 {
                return None;
            }
            let bit = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;
            Some(word_index * u64::BITS as usize + bit)
        })
    })
}

fn coordinates(index: usize) -> (usize, usize, usize) {
    let y = index / CHUNK_AREA;
    let layer_index = index % CHUNK_AREA;
    let z = layer_index / CHUNK_SIZE;
    let x = layer_index % CHUNK_SIZE;

    (x, y, z)
}

fn rotation_index(rotation: TextureRotation) -> u8 {
    match rotation {
        TextureRotation::Degrees0 => 0,
        TextureRotation::Degrees90 => 1,
        TextureRotation::Degrees180 => 2,
        TextureRotation::Degrees270 => 3,
    }
}
