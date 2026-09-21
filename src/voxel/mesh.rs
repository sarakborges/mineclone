use bevy::prelude::*;

use crate::{
    content::block::{BlockRegistry, BlockTextureRotations},
    rendering::block_texture::block_face_material_face,
};

use self::{
    geometry::{face_geometry, is_face_exposed, orient_face_geometry},
    micro_mesh::{
        MicroMeshBuffers, MicroSurface, emit_neighbor_openings, emit_sculpted_faces,
        material_buffer, occludes,
    },
};
use super::{
    block_face::BlockFace,
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, VoxelChunk},
    mesh_lighting::{face_lighting, push_lit_quad, surface_block_srgb},
    microblock::MicroblockMask,
    orientation::orient_face,
    quad::VOXEL_FACE_UVS,
    read::VoxelRead,
    texture_rotation::TextureRotation,
};

pub(crate) mod geometry;
mod micro_mesh;

pub struct ChunkFaceMesh {
    pub block_id: &'static str,
    pub face: BlockFace,
    pub mesh: Mesh,
    pub casts_shadow: bool,
}

pub fn build_chunk_mesh<W, F>(
    world: &W,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    blocks: &BlockRegistry,
    mut tint_at: F,
) -> Vec<ChunkFaceMesh>
where
    W: VoxelRead + ?Sized,
    F: FnMut(IVec3, VoxelCell, &crate::content::block::BlockDefinition) -> [f32; 3],
{
    let mut buffers = MicroMeshBuffers::default();
    let chunk_origin = chunk_coord * CHUNK_SIZE as i32;

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let Some(cell) = chunk.cell_at(x as i32, y as i32, z as i32) else {
                    continue;
                };
                let block = blocks
                    .get(cell.block_id)
                    .unwrap_or_else(|| panic!("missing block definition: {}", cell.block_id));
                let block_is_transparent = block.alpha_blend || block.alpha_cutoff.is_some();
                let world_voxel = chunk_origin + IVec3::new(x as i32, y as i32, z as i32);
                let mut tint = None;
                let mut source_block_srgb = None;
                let block_srgb_for_cell = || {
                    surface_block_srgb(
                        chunk.light_at(x as i32, y as i32, z as i32),
                        block.light_emission > 0,
                    )
                };

                // Geometry and texture identity stay with the original parent.
                // A compact 8^3 mask is expanded only while meshing this voxel.
                if MicroblockMask::is_modified(cell) {
                    let surface = MicroSurface {
                        world,
                        blocks,
                        cell,
                        block,
                        world_voxel,
                        local_voxel: IVec3::new(x as i32, y as i32, z as i32),
                        tint: if block.textures.is_empty() {
                            [1.0, 1.0, 1.0]
                        } else {
                            tint_at(world_voxel, cell, block)
                        },
                        block_srgb: block_srgb_for_cell(),
                    };
                    emit_sculpted_faces(&surface, &mut buffers);
                    continue;
                }

                for block_face in BlockFace::ALL {
                    let face = orient_face(block_face, cell.orientation);
                    let partial_occluder = world
                        .cell_at(world_voxel + face.offset())
                        .filter(|neighbor| MicroblockMask::is_modified(*neighbor))
                        .filter(|neighbor| {
                            let definition = blocks.get(neighbor.block_id).unwrap_or_else(|| {
                                panic!("missing block definition: {}", neighbor.block_id)
                            });
                            occludes(cell.block_id, block, neighbor.block_id, definition)
                        });

                    if partial_occluder.is_none()
                        && !is_face_exposed(
                            world,
                            blocks,
                            cell.block_id,
                            block_is_transparent,
                            world_voxel,
                            face,
                        )
                    {
                        continue;
                    }

                    let tint = *tint.get_or_insert_with(|| {
                        if block.textures.is_empty() {
                            [1.0, 1.0, 1.0]
                        } else {
                            tint_at(world_voxel, cell, block)
                        }
                    });
                    let source_block_srgb =
                        *source_block_srgb.get_or_insert_with(block_srgb_for_cell);

                    if let Some(neighbor) = partial_occluder {
                        let surface = MicroSurface {
                            world,
                            blocks,
                            cell,
                            block,
                            world_voxel,
                            local_voxel: IVec3::new(x as i32, y as i32, z as i32),
                            tint,
                            block_srgb: source_block_srgb,
                        };
                        emit_neighbor_openings(&surface, &mut buffers, face, neighbor);
                        continue;
                    }

                    let texture_rotation =
                        if face_uses_texture_rotation(block.rotate_texture, block_face) {
                            cell.texture_rotation
                        } else {
                            TextureRotation::default()
                        };
                    let geometry = orient_face_geometry(
                        face_geometry(block_face, x, y, z, texture_rotation),
                        cell.orientation,
                        x,
                        y,
                        z,
                    );
                    let lighting = face_lighting(world, world_voxel, face, source_block_srgb);
                    let material_face = block_face_material_face(block_face, block);
                    push_lit_quad(
                        material_buffer(&mut buffers, cell.block_id, block, material_face),
                        geometry.vertices,
                        geometry.normal,
                        geometry.texture_rotation.rotate_uvs(VOXEL_FACE_UVS),
                        tint,
                        lighting,
                    );
                }
            }
        }
    }

    let mut meshes = buffers
        .into_values()
        .filter_map(|entry| {
            entry.buffer.into_mesh().map(|mesh| ChunkFaceMesh {
                block_id: entry.block_id,
                face: entry.face,
                mesh,
                casts_shadow: entry.casts_shadow,
            })
        })
        .collect::<Vec<_>>();

    meshes.sort_by_key(|mesh| (mesh.block_id, face_sort_key(mesh.face), mesh.casts_shadow));
    meshes
}

fn face_sort_key(face: BlockFace) -> u8 {
    match face {
        BlockFace::Right => 0,
        BlockFace::Left => 1,
        BlockFace::Top => 2,
        BlockFace::Bottom => 3,
        BlockFace::Front => 4,
        BlockFace::Back => 5,
    }
}

fn face_uses_texture_rotation(rotations: BlockTextureRotations, face: BlockFace) -> bool {
    match face {
        BlockFace::Right => rotations.right,
        BlockFace::Left => rotations.left,
        BlockFace::Top => rotations.top,
        BlockFace::Bottom => rotations.bottom,
        BlockFace::Front => rotations.front,
        BlockFace::Back => rotations.back,
    }
}
