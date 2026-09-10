mod face;
pub(crate) mod lighting;

use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use face::push_face;
use lighting::{face_lighting, FaceLighting};

use crate::content::block::BlockRegistry;

use super::{
    chunk::{CHUNK_SIZE, VoxelChunk},
    texture_rotation::TextureRotation,
    world::VoxelWorld,
};

const FACE_OVERDRAW: f32 = 0.002;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BlockFace {
    Right,
    Left,
    Top,
    Bottom,
    Front,
    Back,
}

pub struct ChunkFaceMesh {
    pub block_id: &'static str,
    pub face: BlockFace,
    pub mesh: Mesh,
    pub casts_shadow: bool,
}

#[derive(Default)]
struct MeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    light_uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshBuffers {
    fn push(
        &mut self,
        vertices: [[f32; 3]; 4],
        normal: [f32; 3],
        texture_rotation: TextureRotation,
        tint: [f32; 3],
        lighting: FaceLighting,
    ) {
        push_face(
            &mut self.positions,
            &mut self.normals,
            &mut self.uvs,
            &mut self.light_uvs,
            &mut self.colors,
            &mut self.indices,
            vertices,
            normal,
            texture_rotation,
            tint,
            lighting.channels,
            lighting.ambient_occlusion,
        );
    }

    fn into_mesh(self) -> Option<Mesh> {
        if self.positions.is_empty() {
            return None;
        }

        Some(
            Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, self.light_uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
            .with_inserted_indices(Indices::U32(self.indices)),
        )
    }
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
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_origin = chunk_coord * chunk_size;

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let Some(cell) = chunk.cell_at(x as i32, y as i32, z as i32) else {
                    continue;
                };
                let block = blocks
                    .get(cell.block_id)
                    .unwrap_or_else(|| panic!("missing block definition: {}", cell.block_id));
                let casts_shadow = block.casts_shadow;
                let local = IVec3::new(x as i32, y as i32, z as i32);
                let world_voxel = chunk_origin + local;
                let tint = if block.textures.is_empty() {
                    [1.0, 1.0, 1.0]
                } else {
                    tint_at(world_voxel, cell.block_id)
                };
                let x0 = x as f32;
                let y0 = y as f32;
                let z0 = z as f32;
                let x1 = x0 + 1.0;
                let y1 = y0 + 1.0;
                let z1 = z0 + 1.0;
                let e = FACE_OVERDRAW;

                if !world.is_solid(world_voxel + IVec3::X) {
                    buffers
                        .entry((cell.block_id, BlockFace::Right, casts_shadow))
                        .or_default()
                        .push(
                            [
                                [x1, y0 - e, z1 + e],
                                [x1, y0 - e, z0 - e],
                                [x1, y1 + e, z0 - e],
                                [x1, y1 + e, z1 + e],
                            ],
                            [1.0, 0.0, 0.0],
                            TextureRotation::default(),
                            tint,
                            face_lighting(world, world_voxel, BlockFace::Right),
                        );
                }

                if !world.is_solid(world_voxel - IVec3::X) {
                    buffers
                        .entry((cell.block_id, BlockFace::Left, casts_shadow))
                        .or_default()
                        .push(
                            [
                                [x0, y0 - e, z0 - e],
                                [x0, y0 - e, z1 + e],
                                [x0, y1 + e, z1 + e],
                                [x0, y1 + e, z0 - e],
                            ],
                            [-1.0, 0.0, 0.0],
                            TextureRotation::default(),
                            tint,
                            face_lighting(world, world_voxel, BlockFace::Left),
                        );
                }

                if !world.is_solid(world_voxel + IVec3::Y) {
                    buffers
                        .entry((cell.block_id, BlockFace::Top, casts_shadow))
                        .or_default()
                        .push(
                            [
                                [x0 - e, y1, z1 + e],
                                [x1 + e, y1, z1 + e],
                                [x1 + e, y1, z0 - e],
                                [x0 - e, y1, z0 - e],
                            ],
                            [0.0, 1.0, 0.0],
                            cell.texture_rotation,
                            tint,
                            face_lighting(world, world_voxel, BlockFace::Top),
                        );
                }

                if world_voxel.y > 0 && !world.is_solid(world_voxel - IVec3::Y) {
                    buffers
                        .entry((cell.block_id, BlockFace::Bottom, casts_shadow))
                        .or_default()
                        .push(
                            [
                                [x0 - e, y0, z0 - e],
                                [x1 + e, y0, z0 - e],
                                [x1 + e, y0, z1 + e],
                                [x0 - e, y0, z1 + e],
                            ],
                            [0.0, -1.0, 0.0],
                            cell.texture_rotation,
                            tint,
                            face_lighting(world, world_voxel, BlockFace::Bottom),
                        );
                }

                if !world.is_solid(world_voxel + IVec3::Z) {
                    buffers
                        .entry((cell.block_id, BlockFace::Front, casts_shadow))
                        .or_default()
                        .push(
                            [
                                [x0 - e, y0 - e, z1],
                                [x1 + e, y0 - e, z1],
                                [x1 + e, y1 + e, z1],
                                [x0 - e, y1 + e, z1],
                            ],
                            [0.0, 0.0, 1.0],
                            TextureRotation::default(),
                            tint,
                            face_lighting(world, world_voxel, BlockFace::Front),
                        );
                }

                if !world.is_solid(world_voxel - IVec3::Z) {
                    buffers
                        .entry((cell.block_id, BlockFace::Back, casts_shadow))
                        .or_default()
                        .push(
                            [
                                [x1 + e, y0 - e, z0],
                                [x0 - e, y0 - e, z0],
                                [x0 - e, y1 + e, z0],
                                [x1 + e, y1 + e, z0],
                            ],
                            [0.0, 0.0, -1.0],
                            TextureRotation::default(),
                            tint,
                            face_lighting(world, world_voxel, BlockFace::Back),
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
