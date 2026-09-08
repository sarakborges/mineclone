mod face;

use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use face::push_face;

use super::{
    chunk::{VoxelChunk, CHUNK_SIZE},
    texture_rotation::TextureRotation,
    world::VoxelWorld,
};

const TOP_SHADE: f32 = 1.0;
const BOTTOM_SHADE: f32 = 0.78;
const EAST_SHADE: f32 = 0.94;
const WEST_SHADE: f32 = 0.88;
const SOUTH_SHADE: f32 = 0.92;
const NORTH_SHADE: f32 = 0.86;
const SIDE_NORMAL_HORIZONTAL: f32 = 0.8;
const SIDE_NORMAL_UP: f32 = 0.6;
const UNTINTED: [f32; 3] = [1.0, 1.0, 1.0];

#[derive(Clone, Copy)]
pub enum BlockFace {
    Right,
    Left,
    Top,
    Bottom,
    Front,
    Back,
}

pub struct ChunkFaceMesh {
    pub face: BlockFace,
    pub mesh: Mesh,
}

#[derive(Default)]
struct MeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
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
        shade: f32,
    ) {
        push_face(
            &mut self.positions,
            &mut self.normals,
            &mut self.uvs,
            &mut self.colors,
            &mut self.indices,
            vertices,
            normal,
            texture_rotation,
            tint,
            shade,
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
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
            .with_inserted_indices(Indices::U32(self.indices)),
        )
    }
}

pub fn build_chunk_mesh<F>(
    world: &VoxelWorld,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    tint_at: F,
) -> Vec<ChunkFaceMesh>
where
    F: Fn(IVec3) -> [f32; 3],
{
    let mut right = MeshBuffers::default();
    let mut left = MeshBuffers::default();
    let mut top = MeshBuffers::default();
    let mut bottom = MeshBuffers::default();
    let mut front = MeshBuffers::default();
    let mut back = MeshBuffers::default();
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_origin = chunk_coord * chunk_size;

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let Some(cell) = chunk.cell_at(x as i32, y as i32, z as i32) else {
                    continue;
                };

                let local = IVec3::new(x as i32, y as i32, z as i32);
                let world_voxel = chunk_origin + local;
                let grass_tint = tint_at(world_voxel);
                let x0 = x as f32;
                let y0 = y as f32;
                let z0 = z as f32;
                let x1 = x0 + 1.0;
                let y1 = y0 + 1.0;
                let z1 = z0 + 1.0;

                if !world.is_solid(world_voxel + IVec3::X) {
                    right.push(
                        [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]],
                        [SIDE_NORMAL_HORIZONTAL, SIDE_NORMAL_UP, 0.0],
                        TextureRotation::default(),
                        UNTINTED,
                        EAST_SHADE,
                    );
                }

                if !world.is_solid(world_voxel - IVec3::X) {
                    left.push(
                        [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                        [-SIDE_NORMAL_HORIZONTAL, SIDE_NORMAL_UP, 0.0],
                        TextureRotation::default(),
                        UNTINTED,
                        WEST_SHADE,
                    );
                }

                if !world.is_solid(world_voxel + IVec3::Y) {
                    top.push(
                        [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
                        [0.0, 1.0, 0.0],
                        cell.texture_rotation,
                        grass_tint,
                        TOP_SHADE,
                    );
                }

                if world_voxel.y > 0 && !world.is_solid(world_voxel - IVec3::Y) {
                    bottom.push(
                        [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                        [0.0, -1.0, 0.0],
                        cell.texture_rotation,
                        UNTINTED,
                        BOTTOM_SHADE,
                    );
                }

                if !world.is_solid(world_voxel + IVec3::Z) {
                    front.push(
                        [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                        [0.0, SIDE_NORMAL_UP, SIDE_NORMAL_HORIZONTAL],
                        TextureRotation::default(),
                        UNTINTED,
                        SOUTH_SHADE,
                    );
                }

                if !world.is_solid(world_voxel - IVec3::Z) {
                    back.push(
                        [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
                        [0.0, SIDE_NORMAL_UP, -SIDE_NORMAL_HORIZONTAL],
                        TextureRotation::default(),
                        UNTINTED,
                        NORTH_SHADE,
                    );
                }
            }
        }
    }

    [
        (BlockFace::Right, right),
        (BlockFace::Left, left),
        (BlockFace::Top, top),
        (BlockFace::Bottom, bottom),
        (BlockFace::Front, front),
        (BlockFace::Back, back),
    ]
    .into_iter()
    .filter_map(|(face, buffers)| {
        buffers
            .into_mesh()
            .map(|mesh| ChunkFaceMesh { face, mesh })
    })
    .collect()
}
