use crate::content::{layer::LayerFace, object::ObjectPlacementFace};

use super::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, CHUNK_VOLUME, VoxelChunk},
    fluid::FluidCell,
    layer::{AttachedLayer, LayerCell},
    object::ObjectCell,
    texture_rotation::TextureRotation,
};

const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;
const OCCUPANCY_WORDS: usize = CHUNK_VOLUME.div_ceil(u64::BITS as usize);

#[derive(Clone, Copy)]
struct ArchivedLayerCell {
    voxel_index: u16,
    order: u8,
    face: u8,
    layer_id: &'static str,
    rotation: u8,
}

#[derive(Clone, Copy)]
struct ArchivedObjectCell {
    voxel_index: u16,
    face: u8,
    object_id: &'static str,
    rotation: u8,
}

pub struct ArchivedChunk {
    occupancy: [u64; OCCUPANCY_WORDS],
    palette: Vec<VoxelCell>,
    cells: Vec<u16>,
    layers: Vec<ArchivedLayerCell>,
    objects: Vec<ArchivedObjectCell>,
    fluid_occupancy: [u64; OCCUPANCY_WORDS],
    fluid_palette: Vec<FluidCell>,
    fluid_cells: Vec<u16>,
}

impl ArchivedChunk {
    pub(crate) fn from_entries(
        block_entries: impl IntoIterator<Item = (usize, VoxelCell)>,
        layer_entries: impl IntoIterator<Item = (usize, usize, AttachedLayer)>,
        object_entries: impl IntoIterator<Item = (usize, ObjectCell)>,
        fluid_entries: impl IntoIterator<Item = (usize, FluidCell)>,
    ) -> Self {
        let mut occupancy = [0_u64; OCCUPANCY_WORDS];
        let mut palette = Vec::<VoxelCell>::new();
        let mut cells = Vec::new();
        let mut layers = Vec::new();
        let mut objects = Vec::new();
        let mut fluid_occupancy = [0_u64; OCCUPANCY_WORDS];
        let mut fluid_palette = Vec::<FluidCell>::new();
        let mut fluid_cells = Vec::new();

        for (index, cell) in block_entries {
            assert!(index < CHUNK_VOLUME, "archived block index must stay inside the chunk");
            let palette_index = palette
                .iter()
                .position(|candidate| *candidate == cell)
                .unwrap_or_else(|| {
                    palette.push(cell);
                    palette.len() - 1
                });
            assert!(
                palette_index < u16::MAX as usize,
                "chunk block palette cannot exceed {} entries",
                u16::MAX
            );
            occupancy[index / u64::BITS as usize] |= 1_u64 << (index % u64::BITS as usize);
            cells.push(palette_index as u16);
        }

        for (index, order, attached) in layer_entries {
            assert!(index < CHUNK_VOLUME, "archived layer index must stay inside the chunk");
            layers.push(ArchivedLayerCell {
                voxel_index: u16::try_from(index).expect("chunk voxel index must fit in u16"),
                order: u8::try_from(order).expect("layer order must fit in u8"),
                face: attached.face.index(),
                layer_id: attached.cell.layer_id,
                rotation: rotation_index(attached.cell.texture_rotation),
            });
        }
        layers.sort_unstable_by_key(|layer| (layer.voxel_index, layer.order));

        for (index, object) in object_entries {
            assert!(index < CHUNK_VOLUME, "archived object index must stay inside the chunk");
            objects.push(ArchivedObjectCell {
                voxel_index: u16::try_from(index).expect("chunk voxel index must fit in u16"),
                face: object.face.index(),
                object_id: object.object_id,
                rotation: rotation_index(object.rotation),
            });
        }
        objects.sort_unstable_by_key(|object| object.voxel_index);

        for (index, fluid) in fluid_entries {
            assert!(index < CHUNK_VOLUME, "archived fluid index must stay inside the chunk");
            fluid_occupancy[index / u64::BITS as usize] |=
                1_u64 << (index % u64::BITS as usize);
            let palette_index = fluid_palette
                .iter()
                .position(|candidate| *candidate == fluid)
                .unwrap_or_else(|| {
                    fluid_palette.push(fluid);
                    fluid_palette.len() - 1
                });
            assert!(
                palette_index < u16::MAX as usize,
                "chunk fluid palette cannot exceed {} entries",
                u16::MAX
            );
            fluid_cells.push(palette_index as u16);
        }

        Self {
            occupancy,
            palette,
            cells,
            layers,
            objects,
            fluid_occupancy,
            fluid_palette,
            fluid_cells,
        }
    }

    pub fn from_chunk(chunk: &VoxelChunk) -> Self {
        let mut occupancy = [0_u64; OCCUPANCY_WORDS];
        let mut palette = Vec::<VoxelCell>::new();
        let mut cells = Vec::new();
        let mut layers = Vec::new();
        let mut objects = Vec::new();
        let mut fluid_occupancy = [0_u64; OCCUPANCY_WORDS];
        let mut fluid_palette = Vec::<FluidCell>::new();
        let mut fluid_cells = Vec::new();

        for index in 0..CHUNK_VOLUME {
            let (x, y, z) = coordinates(index);
            let (cell, fluid, _) = chunk
                .sample_local(x as i32, y as i32, z as i32)
                .expect("archive coordinates must stay inside the chunk");

            if let Some(cell) = cell {
                let palette_index = palette
                    .iter()
                    .position(|candidate| *candidate == cell)
                    .unwrap_or_else(|| {
                        palette.push(cell);
                        palette.len() - 1
                    });

                assert!(
                    palette_index < u16::MAX as usize,
                    "chunk block palette cannot exceed {} entries",
                    u16::MAX
                );

                occupancy[index / u64::BITS as usize] |= 1_u64 << (index % u64::BITS as usize);
                cells.push(palette_index as u16);
            }

            if let Some(fluid) = fluid {
                fluid_occupancy[index / u64::BITS as usize] |=
                    1_u64 << (index % u64::BITS as usize);
                let palette_index = fluid_palette
                    .iter()
                    .position(|candidate| *candidate == fluid)
                    .unwrap_or_else(|| {
                        fluid_palette.push(fluid);
                        fluid_palette.len() - 1
                    });
                assert!(
                    palette_index < u16::MAX as usize,
                    "chunk fluid palette cannot exceed {} entries",
                    u16::MAX
                );
                fluid_cells.push(palette_index as u16);
            }
        }

        for (index, attached_layers) in chunk.layer_groups() {
            for (order, attached) in attached_layers.iter().copied().enumerate() {
                layers.push(ArchivedLayerCell {
                    voxel_index: u16::try_from(index)
                        .expect("chunk voxel index must fit in u16"),
                    order: u8::try_from(order)
                        .expect("layer order must fit in u8"),
                    face: attached.face.index(),
                    layer_id: attached.cell.layer_id,
                    rotation: rotation_index(attached.cell.texture_rotation),
                });
            }
        }
        layers.sort_unstable_by_key(|layer| (layer.voxel_index, layer.order));
        for (index, object) in chunk.object_entries() {
            objects.push(ArchivedObjectCell {
                voxel_index: u16::try_from(index).expect("chunk voxel index must fit in u16"),
                face: object.face.index(),
                object_id: object.object_id,
                rotation: rotation_index(object.rotation),
            });
        }
        objects.sort_unstable_by_key(|object| object.voxel_index);

        Self {
            occupancy,
            palette,
            cells,
            layers,
            objects,
            fluid_occupancy,
            fluid_palette,
            fluid_cells,
        }
    }

    pub(crate) fn block_entries(&self) -> impl Iterator<Item = (usize, VoxelCell)> + '_ {
        occupied_indices(&self.occupancy)
            .zip(self.cells.iter().copied())
            .map(|(index, palette_index)| (index, self.palette[palette_index as usize]))
    }

    pub(crate) fn layer_entries(
        &self,
    ) -> impl Iterator<Item = (usize, usize, AttachedLayer)> + '_ {
        self.layers.iter().map(|archived| {
            let face = LayerFace::from_index(archived.face)
                .expect("archived layer face must be valid");
            (
                archived.voxel_index as usize,
                archived.order as usize,
                AttachedLayer {
                    face,
                    cell: LayerCell::new(
                        archived.layer_id,
                        TextureRotation::from_quarter_turn(archived.rotation),
                    ),
                },
            )
        })
    }

    pub(crate) fn object_entries(&self) -> impl Iterator<Item = (usize, ObjectCell)> + '_ {
        self.objects.iter().map(|archived| {
            (
                archived.voxel_index as usize,
                ObjectCell {
                    object_id: archived.object_id,
                    face: ObjectPlacementFace::from_index(archived.face)
                        .expect("archived object face must be valid"),
                    rotation: TextureRotation::from_quarter_turn(archived.rotation),
                },
            )
        })
    }

    pub(crate) fn fluid_entries(&self) -> impl Iterator<Item = (usize, FluidCell)> + '_ {
        occupied_indices(&self.fluid_occupancy)
            .zip(self.fluid_cells.iter().copied())
            .map(|(index, palette_index)| (index, self.fluid_palette[palette_index as usize]))
    }

    pub fn restore(&self) -> VoxelChunk {
        let mut chunk = VoxelChunk::empty();

        chunk.edit_initial_blocks(|content| {
            for (index, cell) in self.block_entries() {
                let (x, y, z) = coordinates(index);
                content.set_block(x, y, z, cell);
            }
        });
        for (index, _order, attached) in self.layer_entries() {
            let (x, y, z) = coordinates(index);
            assert!(
                chunk.add_layer(x, y, z, attached.face, attached.cell),
                "archived layer must restore onto its supporting block"
            );
        }
        for (index, object) in self.object_entries() {
            let (x, y, z) = coordinates(index);
            assert!(
                chunk.set_object(x, y, z, object),
                "archived object must restore onto its supporting block"
            );
        }
        if !self.fluid_cells.is_empty() {
            chunk.edit_fluids(|content| {
                for (index, fluid) in self.fluid_entries() {
                    let (x, y, z) = coordinates(index);
                    content.set_fluid(x, y, z, Some(fluid));
                }
            });
        }

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
