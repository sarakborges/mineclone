//! Portable representation of authoritative chunk content.
//! Runtime palette indices, fluid numeric IDs and derived lighting must never
//! cross the disk boundary. New saves use state palettes plus contiguous runs;
//! legacy per-voxel entries remain readable for backward compatibility.
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    block_palette: Vec<DiskBlockState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    block_runs: Vec<DiskRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fluid_palette: Vec<DiskFluidState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fluid_runs: Vec<DiskRun>,
    // Version-1 snapshots stored one JSON object per occupied voxel. Keep
    // these fields readable, but never write them in new snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    blocks: Vec<DiskBlock>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fluids: Vec<DiskFluid>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct DiskBlockState {
    id: String,
    rotation: u8,
    orientation: u8,
    properties: Vec<(String, String)>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct DiskFluidState {
    id: String,
    level: u8,
    source: bool,
    spread_distance: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct DiskRun {
    start: u16,
    len: u16,
    state: u16,
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
        let mut block_palette = Vec::<DiskBlockState>::new();
        let mut block_runs = Vec::<DiskRun>::new();
        let mut fluid_palette = Vec::<DiskFluidState>::new();
        let mut fluid_runs = Vec::<DiskRun>::new();

        for index in 0..CHUNK_VOLUME {
            let (x, y, z) = coordinates(index);
            let (block, fluid, _) = chunk
                .sample_local(x as i32, y as i32, z as i32)
                .expect("disk chunk coordinates must be in range");

            if let Some(cell) = block {
                let mut properties = cell
                    .secondary_properties()
                    .iter_for_save()
                    .map(|(key, value)| (key.to_owned(), value.to_owned()))
                    .collect::<Vec<_>>();
                properties.sort_unstable();
                let state = DiskBlockState {
                    id: cell.block_id.to_owned(),
                    rotation: rotation_index(cell.texture_rotation),
                    orientation: cell.orientation.index(),
                    properties,
                };
                let state_index = palette_index(&mut block_palette, state)?;
                append_run(&mut block_runs, index, state_index)?;
            }

            if let Some(cell) = fluid {
                let definition = fluids.get(cell.fluid_id).ok_or_else(|| {
                    invalid_data(format!("unknown runtime fluid ID {}", cell.fluid_id))
                })?;
                let state = DiskFluidState {
                    id: definition.id.clone(),
                    level: cell.level,
                    source: cell.is_source(),
                    spread_distance: cell.spread_distance(),
                };
                let state_index = palette_index(&mut fluid_palette, state)?;
                append_run(&mut fluid_runs, index, state_index)?;
            }
        }

        Ok(Self {
            coord: [coord.x, coord.y, coord.z],
            block_palette,
            block_runs,
            fluid_palette,
            fluid_runs,
            blocks: Vec::new(),
            fluids: Vec::new(),
        })
    }

    /// Validate the entire chunk before exposing it. New palette/run snapshots
    /// and legacy per-voxel snapshots are both accepted, but mixed encodings
    /// are rejected.
    pub(crate) fn into_chunk(
        self,
        blocks: &BlockRegistry,
        fluids: &FluidRegistry,
    ) -> io::Result<(IVec3, VoxelChunk)> {
        let coord = IVec3::new(self.coord[0], self.coord[1], self.coord[2]);
        if coord.y < 0 {
            return Err(invalid_data("negative chunk Y"));
        }

        let has_compact = !self.block_palette.is_empty()
            || !self.block_runs.is_empty()
            || !self.fluid_palette.is_empty()
            || !self.fluid_runs.is_empty();
        let has_legacy = !self.blocks.is_empty() || !self.fluids.is_empty();
        if has_compact && has_legacy {
            return Err(invalid_data("chunk mixes compact and legacy voxel encodings"));
        }

        if has_compact {
            return decode_compact(
                coord,
                self.block_palette,
                self.block_runs,
                self.fluid_palette,
                self.fluid_runs,
                blocks,
                fluids,
            );
        }

        decode_legacy(coord, self.blocks, self.fluids, blocks, fluids)
    }
}

fn palette_index<T: Eq>(palette: &mut Vec<T>, state: T) -> io::Result<u16> {
    if let Some(index) = palette.iter().position(|candidate| candidate == &state) {
        return u16::try_from(index).map_err(|_| invalid_data("chunk palette index overflow"));
    }
    let index = u16::try_from(palette.len())
        .map_err(|_| invalid_data("chunk palette exceeds supported size"))?;
    palette.push(state);
    Ok(index)
}

fn append_run(runs: &mut Vec<DiskRun>, index: usize, state: u16) -> io::Result<()> {
    let start = u16::try_from(index).map_err(|_| invalid_data("chunk voxel index overflow"))?;
    if let Some(last) = runs.last_mut() {
        let end = last.start as usize + last.len as usize;
        if last.state == state && end == index {
            last.len = last
                .len
                .checked_add(1)
                .ok_or_else(|| invalid_data("chunk run length overflow"))?;
            return Ok(());
        }
    }
    runs.push(DiskRun {
        start,
        len: 1,
        state,
    });
    Ok(())
}

fn decode_compact(
    coord: IVec3,
    block_palette: Vec<DiskBlockState>,
    block_runs: Vec<DiskRun>,
    fluid_palette: Vec<DiskFluidState>,
    fluid_runs: Vec<DiskRun>,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> io::Result<(IVec3, VoxelChunk)> {
    let block_states = block_palette
        .into_iter()
        .map(|state| decode_block_state(state, blocks))
        .collect::<io::Result<Vec<_>>>()?;
    let fluid_states = fluid_palette
        .into_iter()
        .map(|state| decode_fluid_state(state, fluids))
        .collect::<io::Result<Vec<_>>>()?;

    validate_runs(&block_runs, block_states.len())?;
    validate_runs(&fluid_runs, fluid_states.len())?;

    let mut chunk = VoxelChunk::empty();
    chunk.edit_content(|content| {
        for run in &block_runs {
            let cell = block_states[run.state as usize];
            for index in run.start as usize..run.start as usize + run.len as usize {
                let (x, y, z) = coordinates(index);
                content.set_block(x, y, z, Some(cell));
            }
        }
        for run in &fluid_runs {
            let cell = fluid_states[run.state as usize];
            for index in run.start as usize..run.start as usize + run.len as usize {
                let (x, y, z) = coordinates(index);
                content.set_fluid(x, y, z, Some(cell));
            }
        }
    });

    Ok((coord, chunk))
}

fn validate_runs(runs: &[DiskRun], palette_len: usize) -> io::Result<()> {
    let mut previous_end = 0usize;
    for (run_index, run) in runs.iter().enumerate() {
        if run.len == 0 || run.state as usize >= palette_len {
            return Err(invalid_data("invalid chunk run state or length"));
        }
        let start = run.start as usize;
        let end = start
            .checked_add(run.len as usize)
            .ok_or_else(|| invalid_data("chunk run endpoint overflow"))?;
        if end > CHUNK_VOLUME || (run_index > 0 && start < previous_end) {
            return Err(invalid_data("chunk runs must be ordered, non-overlapping and in range"));
        }
        previous_end = end;
    }
    Ok(())
}

fn decode_block_state(state: DiskBlockState, blocks: &BlockRegistry) -> io::Result<VoxelCell> {
    if state.rotation > 3
        || state.orientation > 2
        || state.properties.len() > MAX_PROPERTIES
    {
        return Err(invalid_data(
            "invalid block rotation, orientation or property count",
        ));
    }
    let definition = blocks
        .get(&state.id)
        .ok_or_else(|| invalid_data(format!("missing block definition: {}", state.id)))?;
    validate_properties(&state.properties, definition.can_fragment())?;
    let mut properties = SecondaryProperties::default();
    for (key, value) in state.properties {
        properties.set(&key, &value);
    }
    Ok(
        VoxelCell::oriented(
            intern_block_id(&state.id),
            TextureRotation::from_quarter_turn(state.rotation),
            BlockOrientation::from_index(state.orientation),
        )
        .with_secondary_properties(properties),
    )
}

fn decode_fluid_state(state: DiskFluidState, fluids: &FluidRegistry) -> io::Result<FluidCell> {
    if !(1..=MAX_FLUID_LEVEL).contains(&state.level) {
        return Err(invalid_data("invalid saved fluid level"));
    }
    let fluid_id = fluids
        .id_of(&state.id)
        .ok_or_else(|| invalid_data(format!("missing fluid definition: {}", state.id)))?;
    Ok(FluidCell::with_state(
        fluid_id,
        state.level,
        state.source,
        state.spread_distance,
    ))
}

fn decode_legacy(
    coord: IVec3,
    blocks_on_disk: Vec<DiskBlock>,
    fluids_on_disk: Vec<DiskFluid>,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> io::Result<(IVec3, VoxelChunk)> {
    let mut chunk = VoxelChunk::empty();
    let mut previous = None;
    for entry in blocks_on_disk {
        let index = validate_index(entry.index, previous)?;
        previous = Some(index);
        let cell = decode_block_state(
            DiskBlockState {
                id: entry.id,
                rotation: entry.rotation,
                orientation: entry.orientation,
                properties: entry.properties,
            },
            blocks,
        )?;
        let (x, y, z) = coordinates(index);
        chunk.set_block(x, y, z, Some(cell));
    }

    previous = None;
    for entry in fluids_on_disk {
        let index = validate_index(entry.index, previous)?;
        previous = Some(index);
        let cell = decode_fluid_state(
            DiskFluidState {
                id: entry.id,
                level: entry.level,
                source: entry.source,
                spread_distance: entry.spread_distance,
            },
            fluids,
        )?;
        let (x, y, z) = coordinates(index);
        chunk.set_fluid(x, y, z, Some(cell));
    }

    Ok((coord, chunk))
}

fn validate_properties(
    properties: &[(String, String)],
    can_fragment: bool,
) -> io::Result<()> {
    for (property_index, (key, value)) in properties.iter().enumerate() {
        if key.is_empty()
            || value.is_empty()
            || properties[..property_index]
                .iter()
                .any(|(previous_key, _)| previous_key == key)
        {
            return Err(invalid_data("empty or duplicate secondary property"));
        }
        if key == CHISEL_MASK_PROPERTY
            && (!can_fragment || !MicroblockMask::valid_saved(value))
        {
            return Err(invalid_data("invalid Chisel mask or ineligible block"));
        }
    }
    Ok(())
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

fn rotation_index(rotation: TextureRotation) -> u8 {
    match rotation {
        TextureRotation::Degrees0 => 0,
        TextureRotation::Degrees90 => 1,
        TextureRotation::Degrees180 => 2,
        TextureRotation::Degrees270 => 3,
    }
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
