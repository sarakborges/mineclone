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
            let Some(cell) = chunk.cell_at(x as i32, y as i32, z as i32) else {
                continue;
            };
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

        for index in 0..CHUNK_VOLUME {
            let (x, y, z) = coordinates(index);
            let Some(fluid) = chunk.fluid_at(x as i32, y as i32, z as i32) else {
                continue;
            };

            fluid_occupancy[index / u64::BITS as usize] |= 1_u64 << (index % u64::BITS as usize);
            fluid_cells.push(ArchivedFluidCell {
                fluid_id: fluid.fluid_id,
                level: fluid.level,
                source: fluid.is_source(),
                spread_distance: fluid.spread_distance(),
            });
        }

        Self {
            occupancy,
            palette,
            cells,
            fluid_occupancy,
            fluid_cells,
        }
    }

    pub fn restore(&self) -> VoxelChunk {
        let mut chunk = VoxelChunk::empty();
        let mut archived_cells = self.cells.iter();

        for index in 0..CHUNK_VOLUME {
            let occupied = self.occupancy[index / u64::BITS as usize]
                & (1_u64 << (index % u64::BITS as usize))
                != 0;
            if !occupied {
                continue;
            }

            let archived = archived_cells
                .next()
                .expect("archived chunk occupancy should match archived cells");
            let block_id = self.palette[archived.palette_index as usize];
            let (x, y, z) = coordinates(index);
            chunk.set_block(
                x,
                y,
                z,
                Some(
                    VoxelCell::oriented(
                        block_id,
                        TextureRotation::from_quarter_turn(archived.rotation),
                        BlockOrientation::from_index(archived.orientation),
                    )
                    .with_secondary_properties(archived.secondary_properties),
                ),
            );
        }

        let mut archived_fluids = self.fluid_cells.iter();

        for index in 0..CHUNK_VOLUME {
            let occupied = self.fluid_occupancy[index / u64::BITS as usize]
                & (1_u64 << (index % u64::BITS as usize))
                != 0;
            if !occupied {
                continue;
            }

            let archived = archived_fluids
                .next()
                .expect("archived chunk fluid occupancy should match archived fluid cells");
            let (x, y, z) = coordinates(index);
            chunk.set_fluid(
                x,
                y,
                z,
                Some(FluidCell::with_state(
                    archived.fluid_id,
                    archived.level,
                    archived.source,
                    archived.spread_distance,
                )),
            );
        }

        chunk
    }
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
