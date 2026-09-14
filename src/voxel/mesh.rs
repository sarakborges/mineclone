use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    content::block::{BlockRegistry, BlockTextureRotations},
    rendering::block_texture::block_face_material_face,
};

use self::geometry::{face_geometry, is_face_exposed, orient_face_geometry};
use super::{
    block_face::BlockFace,
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, VoxelChunk},
    mesh_buffer::VoxelMeshBuffer,
    mesh_lighting::{face_lighting, push_lit_quad},
    orientation::orient_face,
    quad::VOXEL_FACE_UVS,
    texture_rotation::TextureRotation,
    world::VoxelWorld,
};

mod geometry;

pub struct ChunkFaceMesh {
    pub block_id: &'static str,
    pub face: BlockFace,
    pub mesh: Mesh,
    pub casts_shadow: bool,
}

pub fn build_chunk_mesh<F>(
    world: &VoxelWorld,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    blocks: &BlockRegistry,
    tint_at: F,
) -> Vec<ChunkFaceMesh>
where
    F: Fn(IVec3, VoxelCell) -> [f32; 3],
{
    let mut buffers = HashMap::<(&'static str, BlockFace, bool), VoxelMeshBuffer>::new();
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
                let world_voxel = chunk_origin + IVec3::new(x as i32, y as i32, z as i32);
                let mut tint = None;

                for block_face in BlockFace::ALL {
                    let face = orient_face(block_face, cell.orientation);
                    if !is_face_exposed(world, blocks, cell.block_id, world_voxel, face) {
                        continue;
                    }

                    let tint = *tint.get_or_insert_with(|| {
                        if block.textures.is_empty() {
                            [1.0, 1.0, 1.0]
                        } else {
                            tint_at(world_voxel, cell)
                        }
                    });
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
                    let lighting = face_lighting(
                        world,
                        world_voxel,
                        face,
                        block.light_emission > 0,
                    );
                    let material_face = block_face_material_face(block_face, block);
                    push_lit_quad(
                        buffers
                            .entry((cell.block_id, material_face, block.casts_shadow))
                            .or_default(),
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
        .into_iter()
        .filter_map(|((block_id, face, casts_shadow), buffer)| {
            buffer.into_mesh().map(|mesh| ChunkFaceMesh {
                block_id,
                face,
                mesh,
                casts_shadow,
            })
        })
        .collect::<Vec<_>>();

    // Stable ordering lets lighting-only remeshes update the existing mesh
    // assets in place instead of destroying and recreating render entities.
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
