//! Greedy meshing of the occupied portions of a macro block. All output goes
//! into the normal chunk material buffers; no entities per microcell are made.

use bevy::prelude::*;
use smallvec::SmallVec;

use crate::{
    content::block::{
        BlockDefinition, BlockLookup, BlockTextureLayer, BlockTextureRotations,
    },
    rendering::block_texture::{
        TerrainTextureTable, block_face_material_face, block_face_texture_layers,
        terrain_array_alpha_signature,
    },
};

use super::ChunkTerrainBatch;
use super::super::{
    block_face::BlockFace,
    cell::VoxelCell,
    mesh_buffer::VoxelMeshBuffer,
    mesh_lighting::{
        ChunkLightingCache, face_lighting_with_cache, push_lit_quad,
    },
    log_variant::{hollow_surface_texture_face, is_hollow_log_id},
    microblock::{MICROBLOCK_EDGE, MicroblockMask, occupied_cell},
    orientation::{orientation_rotation, source_face_for_oriented_face},
    read::VoxelRead,
    texture_rotation::TextureRotation,
};

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(super) enum MaterialBatchKey<'a> {
    Array {
        alpha_cutoff: Option<u32>,
        alpha_blend: bool,
        casts_shadow: bool,
    },
    Legacy {
        layers: &'a [BlockTextureLayer],
        alpha_cutoff: Option<u32>,
        alpha_blend: bool,
        casts_shadow: bool,
    },
}

pub(super) struct MaterialMeshBuffer {
    pub(super) batch: ChunkTerrainBatch,
    pub(super) buffer: VoxelMeshBuffer,
}

pub(super) struct MicroMeshBuffers<'a> {
    entries: SmallVec<[(MaterialBatchKey<'a>, MaterialMeshBuffer); 4]>,
}

impl Default for MicroMeshBuffers<'_> {
    fn default() -> Self {
        Self {
            entries: SmallVec::new(),
        }
    }
}

impl<'a> MicroMeshBuffers<'a> {
    fn buffer_for(
        &mut self,
        key: MaterialBatchKey<'a>,
        batch: ChunkTerrainBatch,
    ) -> &mut VoxelMeshBuffer {
        if let Some(index) = self.entries.iter().position(|(candidate, _)| *candidate == key) {
            return &mut self.entries[index].1.buffer;
        }

        self.entries.push((
            key,
            MaterialMeshBuffer {
                batch,
                buffer: VoxelMeshBuffer::default(),
            },
        ));
        &mut self
            .entries
            .last_mut()
            .expect("material mesh entry was just inserted")
            .1
            .buffer
    }

    pub(super) fn into_values(
        self,
    ) -> impl Iterator<Item = MaterialMeshBuffer> {
        self.entries.into_iter().map(|(_, value)| value)
    }
}

const EDGE: usize = MICROBLOCK_EDGE as usize;
const HOLLOW_LOG_WALL_SCALE: f32 = 0.5;

pub(super) fn material_buffer<'buffer, 'definition>(
    buffers: &'buffer mut MicroMeshBuffers<'definition>,
    block_id: &'static str,
    block: &'definition BlockDefinition,
    face: BlockFace,
) -> &'buffer mut VoxelMeshBuffer {
    let layers = block_face_texture_layers(face, block);
    let alpha_cutoff = block.alpha_cutoff.map(f32::to_bits);
    let array_signature = if layers.len() <= 2 {
        terrain_array_alpha_signature(block)
    } else {
        None
    };
    let (key, batch) = if let Some((alpha_blend, alpha_cutoff)) = array_signature {
        (
            MaterialBatchKey::Array {
                alpha_cutoff,
                alpha_blend,
                casts_shadow: block.casts_shadow,
            },
            ChunkTerrainBatch::Array {
                alpha_cutoff,
                alpha_blend,
                casts_shadow: block.casts_shadow,
            },
        )
    } else {
        (
            MaterialBatchKey::Legacy {
                layers,
                alpha_cutoff,
                alpha_blend: block.alpha_blend,
                casts_shadow: block.casts_shadow,
            },
            ChunkTerrainBatch::Legacy {
                block_id,
                face,
                casts_shadow: block.casts_shadow,
            },
        )
    };

    buffers.buffer_for(key, batch)
}

pub(super) struct MicroSurface<'a, W: VoxelRead + ?Sized> {
    pub(super) world: &'a W,
    pub(super) lighting_cache: Option<&'a ChunkLightingCache>,
    pub(super) cell: VoxelCell,
    pub(super) block: &'a BlockDefinition,
    pub(super) world_voxel: IVec3,
    pub(super) local_voxel: IVec3,
    pub(super) tint: [f32; 3],
    pub(super) block_srgb: [f32; 3],
    pub(super) texture_table: &'a TerrainTextureTable,
}

pub(super) fn emit_sculpted_faces<'a, W: VoxelRead + ?Sized>(
    surface: &MicroSurface<'a, W>,
    blocks: &mut BlockLookup<'a>,
    buffers: &mut MicroMeshBuffers<'a>,
) {
    let shape = MicroblockMask::geometry_for_cell(surface.cell);
    for face in BlockFace::ALL {
        let lighting =
            face_lighting_with_cache(
                surface.lighting_cache,
                surface.world,
                surface.world_voxel,
                face,
                surface.block_srgb,
            );
        for depth in 0..EDGE {
            let mut visible = [false; EDGE * EDGE];
            for v in 0..EDGE {
                for u in 0..EDGE {
                    let position = position_for(face, depth, u, v);
                    if !shape.contains(position) {
                        continue;
                    }
                    let fine = surface.world_voxel * MICROBLOCK_EDGE
                        + IVec3::new(position[0] as i32, position[1] as i32, position[2] as i32);
                    let neighbor = occupied_cell(surface.world, fine + face.offset());
                    let occluded = neighbor.is_some_and(|neighbor| {
                        let definition = blocks.get(neighbor.block_id);
                        occludes(surface.cell.block_id, surface.block, neighbor.block_id, definition)
                    });
                    visible[u + v * EDGE] = !occluded
                        && !(face == BlockFace::Bottom
                            && surface.world_voxel.y == 0
                            && position[1] == 0);
                }
            }
            emit_rectangles(
                surface,
                buffers,
                face,
                depth,
                &mut visible,
                lighting,
            );
        }
    }
}

/// A regular macro face bordering a sculpted solid needs only the openings
/// of the neighbor's boundary mask, not a whole hidden or z-fighting quad.
pub(super) fn emit_neighbor_openings<'a, W: VoxelRead + ?Sized>(
    surface: &MicroSurface<'a, W>,
    buffers: &mut MicroMeshBuffers<'a>,
    face: BlockFace,
    neighbor: VoxelCell,
) {
    let neighbor_mask = MicroblockMask::geometry_for_cell(neighbor);
    let outward = face.offset();
    let depth = if outward.x + outward.y + outward.z > 0 {
        EDGE - 1
    } else {
        0
    };
    let neighbor_depth = EDGE - 1 - depth;
    let mut visible = [false; EDGE * EDGE];
    for v in 0..EDGE {
        for u in 0..EDGE {
            visible[u + v * EDGE] = !neighbor_mask.contains(position_for(
                face,
                neighbor_depth,
                u,
                v,
            ));
        }
    }
    let lighting =
        face_lighting_with_cache(
                surface.lighting_cache,
                surface.world,
                surface.world_voxel,
                face,
                surface.block_srgb,
            );
    emit_rectangles(
        surface,
        buffers,
        face,
        depth,
        &mut visible,
        lighting,
    );
}

pub(super) fn occludes(
    source_id: &str,
    source: &BlockDefinition,
    neighbor_id: &str,
    neighbor: &BlockDefinition,
) -> bool {
    (source_id == neighbor_id && (source.alpha_blend || source.alpha_cutoff.is_some()))
        || (!neighbor.alpha_blend && neighbor.alpha_cutoff.is_none())
}

fn position_for(face: BlockFace, depth: usize, u: usize, v: usize) -> [usize; 3] {
    match face {
        BlockFace::Right | BlockFace::Left => [depth, v, u],
        BlockFace::Top | BlockFace::Bottom => [u, depth, v],
        BlockFace::Front | BlockFace::Back => [u, v, depth],
    }
}

fn emit_rectangles<'a, W: VoxelRead + ?Sized>(
    surface: &MicroSurface<'a, W>,
    buffers: &mut MicroMeshBuffers<'a>,
    face: BlockFace,
    depth: usize,
    visible: &mut [bool; EDGE * EDGE],
    lighting: crate::voxel::mesh_lighting::FaceLighting,
) {
    for v in 0..EDGE {
        for u in 0..EDGE {
            if !visible[u + v * EDGE] {
                continue;
            }
            let mut width = 1;
            while u + width < EDGE && visible[u + width + v * EDGE] {
                width += 1;
            }
            let mut height = 1;
            while v + height < EDGE
                && (u..u + width).all(|column| visible[column + (v + height) * EDGE])
            {
                height += 1;
            }
            for row in v..v + height {
                for column in u..u + width {
                    visible[column + row * EDGE] = false;
                }
            }
            emit_rectangle(
                surface,
                buffers,
                face,
                depth,
                u,
                v,
                width,
                height,
                lighting,
            );
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "a greedy rectangle needs its face, depth and four grid bounds"
)]
fn emit_rectangle<'a, W: VoxelRead + ?Sized>(
    surface: &MicroSurface<'a, W>,
    buffers: &mut MicroMeshBuffers<'a>,
    face: BlockFace,
    depth: usize,
    u: usize,
    v: usize,
    width: usize,
    height: usize,
    lighting: crate::voxel::mesh_lighting::FaceLighting,
) {
    let [min_x, min_y, min_z] = position_for(face, depth, u, v);
    let mut lower = [min_x, min_y, min_z];
    let mut upper = lower;
    match face {
        BlockFace::Right | BlockFace::Left => {
            upper[0] += 1;
            upper[1] += height;
            upper[2] += width;
        }
        BlockFace::Top | BlockFace::Bottom => {
            upper[0] += width;
            upper[1] += 1;
            upper[2] += height;
        }
        BlockFace::Front | BlockFace::Back => {
            upper[0] += width;
            upper[1] += height;
            upper[2] += 1;
        }
    }
    // Both bounds of the normal axis describe the same face plane.
    match face {
        BlockFace::Right => lower[0] = upper[0],
        BlockFace::Left => upper[0] = lower[0],
        BlockFace::Top => lower[1] = upper[1],
        BlockFace::Bottom => upper[1] = lower[1],
        BlockFace::Front => lower[2] = upper[2],
        BlockFace::Back => upper[2] = lower[2],
    }

    let mut vertices = face.unit_vertices().map(|corner| {
        std::array::from_fn(|axis| {
            let coordinate = if corner[axis] == 0.0 {
                lower[axis]
            } else {
                upper[axis]
            };
            surface.local_voxel[axis] as f32 + coordinate as f32 / MICROBLOCK_EDGE as f32
        })
    });
    if is_hollow_log_id(surface.cell.block_id) {
        thin_hollow_log_shell(
            &mut vertices,
            surface.cell.orientation,
            surface.local_voxel,
        );
    }
    let source_face = source_face_for_oriented_face(face, surface.cell.orientation);
    let rotation = if rotates_texture(surface.block.rotate_texture, source_face) {
        surface.cell.texture_rotation
    } else {
        TextureRotation::default()
    };
    let inverse_orientation = orientation_rotation(surface.cell.orientation).inverse();
    let uvs = vertices.map(|vertex| {
        let local = Vec3::from_array(vertex) - surface.local_voxel.as_vec3();
        let oriented = Vec3::splat(0.5)
            + inverse_orientation * (local - Vec3::splat(0.5));
        rotate_macro_uv(macro_uv(source_face, oriented), rotation)
    });
    let material_face = block_face_material_face(
        hollow_surface_texture_face(surface.cell.block_id, source_face, !is_macro_boundary(face, depth)),
        surface.block,
    );
    let material_code = surface
        .texture_table
        .encoded_layers(block_face_texture_layers(material_face, surface.block))
        .unwrap_or(0.0);
    push_lit_quad(
        material_buffer(
            buffers,
            surface.cell.block_id,
            surface.block,
            material_face,
        ),
        vertices,
        face.normal(),
        uvs,
        surface.tint,
        lighting,
        material_code,
    );
}

fn thin_hollow_log_shell(
    vertices: &mut [[f32; 3]; 4],
    orientation: crate::content::block_orientation::BlockOrientation,
    local_voxel: IVec3,
) {
    use crate::content::block_orientation::BlockOrientation;

    let radial_axes = match orientation {
        BlockOrientation::Y => [0, 2],
        BlockOrientation::Z => [0, 1],
        BlockOrientation::X => [1, 2],
    };
    let origin = local_voxel.as_vec3().to_array();

    for vertex in vertices {
        for axis in radial_axes {
            let local = vertex[axis] - origin[axis];
            let distance_from_edge = local.min(1.0 - local);
            let scaled_distance = distance_from_edge * HOLLOW_LOG_WALL_SCALE;
            vertex[axis] = origin[axis]
                + if local <= 0.5 {
                    scaled_distance
                } else {
                    1.0 - scaled_distance
                };
        }
    }
}

fn is_macro_boundary(face: BlockFace, depth: usize) -> bool {
    match face {
        BlockFace::Right | BlockFace::Top | BlockFace::Front => depth == EDGE - 1,
        BlockFace::Left | BlockFace::Bottom | BlockFace::Back => depth == 0,
    }
}

fn macro_uv(face: BlockFace, point: Vec3) -> [f32; 2] {
    match face {
        BlockFace::Right => [1.0 - point.z, 1.0 - point.y],
        BlockFace::Left => [point.z, 1.0 - point.y],
        BlockFace::Top => [point.x, point.z],
        BlockFace::Bottom => [point.x, 1.0 - point.z],
        BlockFace::Front => [point.x, 1.0 - point.y],
        BlockFace::Back => [1.0 - point.x, 1.0 - point.y],
    }
}

fn rotate_macro_uv([u, v]: [f32; 2], rotation: TextureRotation) -> [f32; 2] {
    match rotation {
        TextureRotation::Degrees0 => [u, v],
        TextureRotation::Degrees90 => [1.0 - v, u],
        TextureRotation::Degrees180 => [1.0 - u, 1.0 - v],
        TextureRotation::Degrees270 => [v, 1.0 - u],
    }
}

fn rotates_texture(rotations: BlockTextureRotations, face: BlockFace) -> bool {
    match face {
        BlockFace::Right => rotations.right,
        BlockFace::Left => rotations.left,
        BlockFace::Top => rotations.top,
        BlockFace::Bottom => rotations.bottom,
        BlockFace::Front => rotations.front,
        BlockFace::Back => rotations.back,
    }
}
