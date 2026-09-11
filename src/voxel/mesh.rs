mod buffer;
mod face;
mod geometry;
pub(crate) mod lighting;

use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::{
    block::{BlockRegistry, BlockTextureRotations},
    block_orientation::BlockOrientation,
};

use self::{
    buffer::MeshBuffers,
    geometry::{face_geometry, is_face_exposed, orient_face_geometry},
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

    fn oriented(self, orientation: BlockOrientation) -> Self {
        match orientation {
            BlockOrientation::Y => self,
            BlockOrientation::Z => match self {
                Self::Right => Self::Right,
                Self::Left => Self::Left,
                Self::Top => Self::Front,
                Self::Bottom => Self::Back,
                Self::Front => Self::Bottom,
                Self::Back => Self::Top,
            },
            BlockOrientation::X => match self {
                Self::Right => Self::Bottom,
                Self::Left => Self::Top,
                Self::Top => Self::Right,
                Self::Bottom => Self::Left,
                Self::Front => Self::Front,
                Self::Back => Self::Back,
            },
        }
    }
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

                for block_face in BlockFace::ALL {
                    let face = block_face.oriented(cell.orientation);
                    if !is_face_exposed(world, blocks, cell.block_id, world_voxel, face) {
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
                    buffers
                        .entry((cell.block_id, block_face, block.casts_shadow))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axial_orientation_moves_top_face_to_selected_axis() {
        assert_eq!(
            BlockFace::Top.oriented(BlockOrientation::Y),
            BlockFace::Top
        );
        assert_eq!(
            BlockFace::Top.oriented(BlockOrientation::Z),
            BlockFace::Front
        );
        assert_eq!(
            BlockFace::Top.oriented(BlockOrientation::X),
            BlockFace::Right
        );
    }
}
