//! Portable representation of authoritative chunk content.
//! Runtime palette indices, fluid numeric IDs and derived lighting must never
//! cross the disk boundary. The disk schema uses state palettes plus contiguous runs.
use std::io;

use bevy::prelude::IVec3;
use serde::{Deserialize, Serialize};

use crate::content::{
    block::BlockRegistry, block_id::intern_block_id, block_orientation::BlockOrientation,
    fluid::FluidRegistry,
    layer::{LayerFace, LayerRegistry},
    layer_id::intern_layer_id,
};

use super::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, CHUNK_VOLUME, VoxelChunk},
    chunk_archive::ArchivedChunk,
    fluid::{FluidCell, MAX_FLUID_LEVEL},
    layer::{AttachedLayer, LayerCell, MAX_LAYERS_PER_VOXEL},
    microblock::{CHISEL_MASK_PROPERTY, MicroblockMask},
    secondary_properties::SecondaryProperties,
    texture_rotation::TextureRotation,
};

const MAX_PROPERTIES: usize = 8;
const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiskChunk {
    coord: [i32; 3],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    block_palette: Vec<DiskBlockState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    block_runs: Vec<DiskRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    layers: Vec<DiskLayerState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fluid_palette: Vec<DiskFluidState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fluid_runs: Vec<DiskRun>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DiskBlockState {
    id: String,
    rotation: u8,
    orientation: u8,
    properties: Vec<(String, String)>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DiskLayerState {
    voxel: u16,
    order: u8,
    face: u8,
    id: String,
    rotation: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DiskFluidState {
    id: String,
    level: u8,
    source: bool,
    spread_distance: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DiskRun {
    start: u16,
    len: u16,
    state: u16,
}

struct DiskChunkBuilder {
    coord: [i32; 3],
    block_palette: Vec<DiskBlockState>,
    block_runs: Vec<DiskRun>,
    layers: Vec<DiskLayerState>,
    fluid_palette: Vec<DiskFluidState>,
    fluid_runs: Vec<DiskRun>,
}

impl DiskChunkBuilder {
    fn new(coord: IVec3) -> Self {
        Self {
            coord: [coord.x, coord.y, coord.z],
            block_palette: Vec::new(),
            block_runs: Vec::new(),
            layers: Vec::new(),
            fluid_palette: Vec::new(),
            fluid_runs: Vec::new(),
        }
    }

    fn push_block(&mut self, index: usize, cell: VoxelCell) -> io::Result<()> {
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
        let state_index = palette_index(&mut self.block_palette, state)?;
        append_run(&mut self.block_runs, index, state_index)
    }

    fn push_layer(
        &mut self,
        index: usize,
        order: usize,
        attached: AttachedLayer,
    ) -> io::Result<()> {
        self.layers.push(DiskLayerState {
            voxel: u16::try_from(index)
                .map_err(|_| invalid_data("chunk layer voxel index overflow"))?,
            order: u8::try_from(order)
                .map_err(|_| invalid_data("chunk layer order overflow"))?,
            face: attached.face.index(),
            id: attached.cell.layer_id.to_owned(),
            rotation: rotation_index(attached.cell.texture_rotation),
        });
        Ok(())
    }

    fn push_fluid(
        &mut self,
        index: usize,
        cell: FluidCell,
        fluids: &FluidRegistry,
    ) -> io::Result<()> {
        let definition = fluids.get(cell.fluid_id).ok_or_else(|| {
            invalid_data(format!("unknown runtime fluid ID {}", cell.fluid_id))
        })?;
        let state = DiskFluidState {
            id: definition.id.clone(),
            level: cell.level,
            source: cell.is_source(),
            spread_distance: cell.spread_distance(),
        };
        let state_index = palette_index(&mut self.fluid_palette, state)?;
        append_run(&mut self.fluid_runs, index, state_index)
    }

    fn finish(mut self) -> DiskChunk {
        self.layers
            .sort_unstable_by_key(|layer| (layer.voxel, layer.order));
        DiskChunk {
            coord: self.coord,
            block_palette: self.block_palette,
            block_runs: self.block_runs,
            layers: self.layers,
            fluid_palette: self.fluid_palette,
            fluid_runs: self.fluid_runs,
        }
    }
}

impl DiskChunk {
    pub(crate) fn coord(&self) -> io::Result<IVec3> {
        let coord = IVec3::new(self.coord[0], self.coord[1], self.coord[2]);
        if coord.y < 0 {
            return Err(invalid_data("negative chunk Y"));
        }
        Ok(coord)
    }

    pub(crate) fn from_chunk(
        coord: IVec3,
        chunk: &VoxelChunk,
        fluids: &FluidRegistry,
    ) -> io::Result<Self> {
        let mut builder = DiskChunkBuilder::new(coord);

        for index in 0..CHUNK_VOLUME {
            let (x, y, z) = coordinates(index);
            let (block, fluid, _) = chunk
                .sample_local(x as i32, y as i32, z as i32)
                .expect("disk chunk coordinates must be in range");

            if let Some(cell) = block {
                builder.push_block(index, cell)?;
            }
            if let Some(cell) = fluid {
                builder.push_fluid(index, cell, fluids)?;
            }
        }

        for (index, attached_layers) in chunk.layer_groups() {
            for (order, attached) in attached_layers.iter().copied().enumerate() {
                builder.push_layer(index, order, attached)?;
            }
        }

        Ok(builder.finish())
    }

    pub(crate) fn from_archived_chunk(
        coord: IVec3,
        chunk: &ArchivedChunk,
        fluids: &FluidRegistry,
    ) -> io::Result<Self> {
        let mut builder = DiskChunkBuilder::new(coord);

        for (index, cell) in chunk.block_entries() {
            builder.push_block(index, cell)?;
        }
        for (index, order, attached) in chunk.layer_entries() {
            builder.push_layer(index, order, attached)?;
        }
        for (index, cell) in chunk.fluid_entries() {
            builder.push_fluid(index, cell, fluids)?;
        }

        Ok(builder.finish())
    }

    /// Validate the entire current-format chunk before exposing it.
    pub(crate) fn into_archived_chunk(
        self,
        blocks: &BlockRegistry,
        layers: &LayerRegistry,
        fluids: &FluidRegistry,
    ) -> io::Result<(IVec3, ArchivedChunk)> {
        let coord = self.coord()?;
        decode_archived_compact(
            coord,
            self.block_palette,
            self.block_runs,
            self.layers,
            self.fluid_palette,
            self.fluid_runs,
            blocks,
            layers,
            fluids,
        )
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

#[expect(
    clippy::too_many_arguments,
    reason = "direct archive decoding keeps the independent disk channels explicit"
)]
fn decode_archived_compact(
    coord: IVec3,
    block_palette: Vec<DiskBlockState>,
    block_runs: Vec<DiskRun>,
    layer_states: Vec<DiskLayerState>,
    fluid_palette: Vec<DiskFluidState>,
    fluid_runs: Vec<DiskRun>,
    blocks: &BlockRegistry,
    layers: &LayerRegistry,
    fluids: &FluidRegistry,
) -> io::Result<(IVec3, ArchivedChunk)> {
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
    let layer_entries = decode_layer_entries(&block_runs, layer_states, layers)?;

    let block_entries = block_runs.iter().flat_map(|run| {
        let cell = block_states[run.state as usize];
        (run.start as usize..run.start as usize + run.len as usize)
            .map(move |index| (index, cell))
    });
    let fluid_entries = fluid_runs.iter().flat_map(|run| {
        let cell = fluid_states[run.state as usize];
        (run.start as usize..run.start as usize + run.len as usize)
            .map(move |index| (index, cell))
    });

    Ok((
        coord,
        ArchivedChunk::from_entries(block_entries, layer_entries, fluid_entries),
    ))
}

fn decode_layer_entries(
    block_runs: &[DiskRun],
    states: Vec<DiskLayerState>,
    layers: &LayerRegistry,
) -> io::Result<Vec<(usize, usize, AttachedLayer)>> {
    let mut previous_voxel = None;
    let mut expected_order = 0_u8;
    let mut current_layers = Vec::<(LayerFace, &'static str)>::new();
    let mut entries = Vec::with_capacity(states.len());

    for state in states {
        if state.voxel as usize >= CHUNK_VOLUME
            || state.rotation > 3
            || state.order as usize >= MAX_LAYERS_PER_VOXEL
        {
            return Err(invalid_data("invalid saved layer voxel, order or rotation"));
        }

        match previous_voxel {
            Some(previous) if state.voxel < previous => {
                return Err(invalid_data("saved layers must be ordered by voxel and layer order"));
            }
            Some(previous) if state.voxel == previous => {
                if state.order != expected_order {
                    return Err(invalid_data("saved layer order must be contiguous per voxel"));
                }
            }
            _ => {
                if state.order != 0 {
                    return Err(invalid_data("first saved layer in a voxel must have order zero"));
                }
                current_layers.clear();
            }
        }

        let face = LayerFace::from_index(state.face)
            .ok_or_else(|| invalid_data("invalid saved layer face"))?;
        let definition = layers
            .get(&state.id)
            .ok_or_else(|| invalid_data(format!("missing layer definition: {}", state.id)))?;
        if !definition.supports_face(face) {
            return Err(invalid_data(format!(
                "layer {} does not support saved face {:?}",
                state.id, face
            )));
        }

        let voxel = state.voxel as usize;
        if !run_contains_index(block_runs, voxel) {
            return Err(invalid_data("saved layer is missing its supporting block"));
        }

        let layer_id = intern_layer_id(&state.id);
        if current_layers
            .iter()
            .any(|(existing_face, existing_id)| *existing_face == face && *existing_id == layer_id)
        {
            return Err(invalid_data("duplicate saved layer on the same voxel face"));
        }
        current_layers.push((face, layer_id));

        entries.push((
            voxel,
            state.order as usize,
            AttachedLayer {
                face,
                cell: LayerCell {
                    layer_id,
                    texture_rotation: TextureRotation::from_quarter_turn(state.rotation),
                },
            },
        ));

        previous_voxel = Some(state.voxel);
        expected_order = state
            .order
            .checked_add(1)
            .ok_or_else(|| invalid_data("saved layer order overflow"))?;
    }

    Ok(entries)
}

fn run_contains_index(runs: &[DiskRun], index: usize) -> bool {
    runs.iter().any(|run| {
        let start = run.start as usize;
        index >= start && index < start + run.len as usize
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{
        chunk_archive::ArchivedChunk,
        texture_rotation::TextureRotation,
    };

    #[test]
    fn archived_chunk_serializes_identically_without_runtime_restore() {
        let coord = IVec3::new(2, 3, -4);
        let fluids = FluidRegistry::default();
        let mut chunk = VoxelChunk::empty();
        chunk.set_block(
            1,
            2,
            3,
            Some(VoxelCell::new("asteria:test", TextureRotation::Degrees90)),
        );
        chunk.set_block(
            4,
            5,
            6,
            Some(VoxelCell::new("asteria:other", TextureRotation::Degrees180)),
        );

        let archived = ArchivedChunk::from_chunk(&chunk);
        let resident_disk = DiskChunk::from_chunk(coord, &chunk, &fluids).unwrap();
        let archived_disk =
            DiskChunk::from_archived_chunk(coord, &archived, &fluids).unwrap();

        assert_eq!(
            serde_json::to_string(&resident_disk).unwrap(),
            serde_json::to_string(&archived_disk).unwrap()
        );
    }
}
