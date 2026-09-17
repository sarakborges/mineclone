//! Greedy meshing of the occupied portions of a macro block. All output goes
//! into the normal chunk material buffers; no entities per microcell are made.

use bevy::{platform::collections::HashMap, prelude::*};

use crate::{
    content::block::{BlockDefinition, BlockRegistry, BlockTextureRotations},
    rendering::block_texture::block_face_material_face,
};

use super::super::{
    block_face::BlockFace,
    cell::VoxelCell,
    mesh_buffer::VoxelMeshBuffer,
    mesh_lighting::{face_lighting, push_lit_quad},
    microblock::{MICROBLOCK_EDGE, MicroblockMask, occupied_cell},
    orientation::{orientation_rotation, source_face_for_oriented_face},
    read::VoxelRead,
    texture_rotation::TextureRotation,
};

pub(super) type MicroMeshBuffers = HashMap<(&'static str, BlockFace, bool), VoxelMeshBuffer>;
const EDGE: usize = MICROBLOCK_EDGE as usize;

pub(super) struct MicroSurface<'a, W: VoxelRead + ?Sized> {
    pub(super) world: &'a W,
    pub(super) blocks: &'a BlockRegistry,
    pub(super) cell: VoxelCell,
    pub(super) block: &'a BlockDefinition,
    pub(super) world_voxel: IVec3,
    pub(super) local_voxel: IVec3,
    pub(super) tint: [f32; 3],
    pub(super) block_srgb: [f32; 3],
}

pub(super) fn emit_sculpted_faces<W: VoxelRead + ?Sized>(
    surface: &MicroSurface<'_, W>,
    buffers: &mut MicroMeshBuffers,
) {
    let shape = MicroblockMask::from_cell(surface.cell);
    for face in BlockFace::ALL {
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
                        let definition = surface.blocks.get(neighbor.block_id).unwrap_or_else(|| {
                            panic!("missing block definition: {}", neighbor.block_id)
                        });
                        occludes(surface.cell.block_id, surface.block, neighbor.block_id, definition)
                    });
                    visible[u + v * EDGE] = !occluded
                        && !(face == BlockFace::Bottom
                            && surface.world_voxel.y == 0
                            && position[1] == 0);
                }
            }
            emit_rectangles(surface, buffers, face, depth, &mut visible);
        }
    }
}

/// A regular macro face bordering a sculpted solid needs only the openings
/// of the neighbor's boundary mask, not a whole hidden or z-fighting quad.
pub(super) fn emit_neighbor_openings<W: VoxelRead + ?Sized>(
    surface: &MicroSurface<'_, W>,
    buffers: &mut MicroMeshBuffers,
    face: BlockFace,
    neighbor: VoxelCell,
) {
    let neighbor_mask = MicroblockMask::from_cell(neighbor);
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
    emit_rectangles(surface, buffers, face, depth, &mut visible);
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

fn emit_rectangles<W: VoxelRead + ?Sized>(
    surface: &MicroSurface<'_, W>,
    buffers: &mut MicroMeshBuffers,
    face: BlockFace,
    depth: usize,
    visible: &mut [bool; EDGE * EDGE],
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
            emit_rectangle(surface, buffers, face, depth, u, v, width, height);
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "a greedy rectangle needs its face, depth and four grid bounds"
)]
fn emit_rectangle<W: VoxelRead + ?Sized>(
    surface: &MicroSurface<'_, W>,
    buffers: &mut MicroMeshBuffers,
    face: BlockFace,
    depth: usize,
    u: usize,
    v: usize,
    width: usize,
    height: usize,
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

    let vertices = face.unit_vertices().map(|corner| {
        std::array::from_fn(|axis| {
            let coordinate = if corner[axis] == 0.0 {
                lower[axis]
            } else {
                upper[axis]
            };
            surface.local_voxel[axis] as f32 + coordinate as f32 / MICROBLOCK_EDGE as f32
        })
    });
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
    let material_face = block_face_material_face(source_face, surface.block);
    let lighting = face_lighting(surface.world, surface.world_voxel, face, surface.block_srgb);
    push_lit_quad(
        buffers
            .entry((surface.cell.block_id, material_face, surface.block.casts_shadow))
            .or_default(),
        vertices,
        face.normal(),
        uvs,
        surface.tint,
        lighting,
    );
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
