//! Portable, version-independent representation of a modified chunk.
//! Runtime palette indices, fluid numeric IDs and derived lighting must never
//! cross the disk boundary.
use std::io;

use bevy::prelude::IVec3;
use serde::{Deserialize, Serialize};

use crate::content::{
    block::BlockRegistry, block_id::intern_block_id, block_orientation::BlockOrientation,
    fluid::FluidRegistry,
};

use super::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, CHUNK_VOLUME, VoxelChunk},
    fluid::{FluidCell, MAX_FLUID_LEVEL},
    microblock::{CHISEL_MASK_PROPERTY, MicroblockMask},
    secondary_properties::SecondaryProperties,
    texture_rotation::TextureRotation,
};

const MAX_PROPERTIES: usize = 8;
const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DiskChunk {
    pub(crate) coord: [i32; 3],
    blocks: Vec<DiskBlock>,
    fluids: Vec<DiskFluid>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DiskBlock {
    index: u16,
    id: String,
    rotation: u8,
    orientation: u8,
    properties: Vec<(String, String)>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DiskFluid {
    index: u16,
    id: String,
    level: u8,
    source: bool,
    spread_distance: u16,
}

impl DiskChunk {
    pub(crate) fn from_chunk(
        coord: IVec3,
        chunk: &VoxelChunk,
        fluids: &FluidRegistry,
    ) -> io::Result<Self> {
        let mut blocks = Vec::new();
        let mut saved_fluids = Vec::new();
        for index in 0..CHUNK_VOLUME {
            let (x, y, z) = coordinates(index);
            let (block, fluid, _) = chunk
                .sample_local(x as i32, y as i32, z as i32)
                .expect("disk chunk coordinates must be in range");
            // Save the complete cell state, including private Chisel masks;
            // public presentation intentionally uses a different iterator.
            if let Some(cell) = block {
                let mut properties = cell
                    .secondary_properties()
                    .iter_for_save()
                    .map(|(key, value)| (key.to_owned(), value.to_owned()))
                    .collect::<Vec<_>>();
                properties.sort_unstable();
                blocks.push(DiskBlock {
                    index: index as u16,
                    id: cell.block_id.to_owned(),
                    rotation: match cell.texture_rotation {
                        TextureRotation::Degrees0 => 0,
                        TextureRotation::Degrees90 => 1,
                        TextureRotation::Degrees180 => 2,
                        TextureRotation::Degrees270 => 3,
                    },
                    orientation: cell.orientation.index(),
                    properties,
                });
            }
            if let Some(cell) = fluid {
                let definition = fluids.get(cell.fluid_id).ok_or_else(|| {
                    invalid_data(format!("unknown runtime fluid ID {}", cell.fluid_id))
                })?;
                saved_fluids.push(DiskFluid {
                    index: index as u16,
                    id: definition.id.clone(),
                    level: cell.level,
                    source: cell.is_source(),
                    spread_distance: cell.spread_distance(),
                });
            }
        }
        Ok(Self {
            coord: [coord.x, coord.y, coord.z],
            blocks,
            fluids: saved_fluids,
        })
    }

    /// Validate the *entire* chunk before mutating the world; malformed files
    /// are errors, not panics in runtime cell constructors. An invalid Chisel
    /// mask rejects this save generation rather than restoring a full cube.
    pub(crate) fn into_chunk(
        self,
        blocks: &BlockRegistry,
        fluids: &FluidRegistry,
    ) -> io::Result<(IVec3, VoxelChunk)> {
        let coord = IVec3::new(self.coord[0], self.coord[1], self.coord[2]);
        if coord.y < 0 {
            return Err(invalid_data("negative chunk Y"));
        }
        let mut chunk = VoxelChunk::empty();
        let mut previous = None;
        for entry in self.blocks {
            let index = validate_index(entry.index, previous)?;
            previous = Some(index);
            if entry.rotation > 3 || entry.orientation > 2 || entry.properties.len() > MAX_PROPERTIES {
                return Err(invalid_data("invalid block rotation, orientation or property count"));
            }
            let definition = blocks.get(&entry.id).ok_or_else(|| {
                invalid_data(format!("missing block definition: {}", entry.id))
            })?;
            // At most eight properties: validate in-place rather than allocate
            // a HashSet per saved block. Reject duplicates and damaged masks
            // before interning any properties or exposing the restored world.
            for (property_index, (key, value)) in entry.properties.iter().enumerate() {
                if key.is_empty()
                    || value.is_empty()
                    || entry.properties[..property_index]
                        .iter()
                        .any(|(previous_key, _)| previous_key == key)
                {
                    return Err(invalid_data("empty or duplicate secondary property"));
                }
                if key == CHISEL_MASK_PROPERTY
                    && (!definition.can_fragment() || !MicroblockMask::valid_saved(value))
                {
                    return Err(invalid_data("invalid Chisel mask or ineligible block"));
                }
            }
            let mut properties = SecondaryProperties::default();
            for (key, value) in entry.properties {
                properties.set(&key, &value);
            }
            let (x, y, z) = coordinates(index);
            chunk.set_block(
                x,
                y,
                z,
                Some(
                    VoxelCell::oriented(
                        intern_block_id(&entry.id),
                        TextureRotation::from_quarter_turn(entry.rotation),
                        BlockOrientation::from_index(entry.orientation),
                    )
                    .with_secondary_properties(properties),
                ),
            );
        }
        previous = None;
        for entry in self.fluids {
            let index = validate_index(entry.index, previous)?;
            previous = Some(index);
            if !(1..=MAX_FLUID_LEVEL).contains(&entry.level) {
                return Err(invalid_data("invalid saved fluid level"));
            }
            let fluid_id = fluids.id_of(&entry.id).ok_or_else(|| {
                invalid_data(format!("missing fluid definition: {}", entry.id))
            })?;
            let (x, y, z) = coordinates(index);
            chunk.set_fluid(
                x,
                y,
                z,
                Some(FluidCell::with_state(
                    fluid_id,
                    entry.level,
                    entry.source,
                    entry.spread_distance,
                )),
            );
        }
        Ok((coord, chunk))
    }
}

fn validate_index(index: u16, previous: Option<usize>) -> io::Result<usize> {
    let index = index as usize;
    if index >= CHUNK_VOLUME || previous.is_some_and(|previous| index <= previous) {
        return Err(invalid_data("saved voxel indexes must be in range and strictly increasing"));
    }
    Ok(index)
}

fn coordinates(index: usize) -> (usize, usize, usize) {
    let y = index / CHUNK_AREA;
    let z = index % CHUNK_AREA / CHUNK_SIZE;
    let x = index % CHUNK_SIZE;
    (x, y, z)
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
