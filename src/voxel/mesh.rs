mod buffer;
mod face;
mod geometry;
pub(crate) mod lighting;

use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::{block::{BlockRegistry, BlockTextureRotations}};

use self::{
    buffer::MeshBuffers,
    geometry::{face_geometry, is_face_exposed},
    lighting::face_lighting,
};
use super::{
    chunk::{CHUNK_SIZE, VoxelChunk},
    texture_rotation::TextureRotation,
    world::VoxelWorld,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BlockFace {
    Right,
    Left,
    Top,
    Bottom,
    Front,
    Back,
}

impl BlockFace {
    const ALL: [Self; 6] = [
        Self::Right,
        Self::Left,
        Self::Top,
        Self::Bottom,
        Self::Front,
        Self::Back,
    ];
}

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
    F: Fn(IVec3, &'static str) -> [f32; 3],
{
    let mut buffers = HashMap::<(&'static str, BlockFace, bool), MeshBuffers>::new();
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
                let tint = if block.textures.is_empty() {
                    [1.0, 1.0, 1.0]
                } else {
                    tint_at(world_voxel, cell.block_id)
                };

                for face in BlockFace::ALL {
                    if !is_face_exposed(world, world_voxel, face) {
                        continue;
                    }

                    let texture_rotation = if face_uses_texture_rotation(block.rotate_texture, face) {
                        cell.texture_rotation
                    } else {
                        TextureRotation::default()
                    };
                    let geometry = face_geometry(face, x, y, z, texture_rotation);
                    buffers
                        .entry((cell.block_id, face, block.casts_shadow))
                        .or_default()
                        .push(
                            geometry.vertices,
                            geometry.normal,
                            geometry.texture_rotation,
                            tint,
                            face_lighting(world, world_voxel, face),
                        );
                }
            }
        }
    }

    buffers
        .into_iter()
        .filter_map(|((block_id, face, casts_shadow), buffers)| {
            buffers.into_mesh().map(|mesh| ChunkFaceMesh {
                block_id,
                face,
                mesh,
                casts_shadow,
            })
        })
        .collect()
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
