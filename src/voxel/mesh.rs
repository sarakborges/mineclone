mod face;

use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use face::push_face;

use super::{
    chunk::{CHUNK_SIZE, VoxelChunk},
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
const SKY_LIGHT_SEARCH_RADIUS: i32 = 4;
const SKY_LIGHT_LATERAL_ATTENUATION: f32 = 0.18;
const ENCLOSED_SKY_LIGHT: f32 = 0.08;
const SKY_LIGHT_DIRECTIONS: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

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
        skylight: f32,
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
            skylight,
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
    let mut sky_heights = HashMap::new();
    let max_loaded_chunk_y = world.highest_loaded_chunk_y();
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
                        grass_tint,
                        EAST_SHADE,
                        skylight_at(
                            world,
                            &mut sky_heights,
                            max_loaded_chunk_y,
                            world_voxel + IVec3::X,
                        ),
                    );
                }

                if !world.is_solid(world_voxel - IVec3::X) {
                    left.push(
                        [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                        [-SIDE_NORMAL_HORIZONTAL, SIDE_NORMAL_UP, 0.0],
                        TextureRotation::default(),
                        grass_tint,
                        WEST_SHADE,
                        skylight_at(
                            world,
                            &mut sky_heights,
                            max_loaded_chunk_y,
                            world_voxel - IVec3::X,
                        ),
                    );
                }

                if !world.is_solid(world_voxel + IVec3::Y) {
                    top.push(
                        [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
                        [0.0, 1.0, 0.0],
                        cell.texture_rotation,
                        grass_tint,
                        TOP_SHADE,
                        skylight_at(
                            world,
                            &mut sky_heights,
                            max_loaded_chunk_y,
                            world_voxel + IVec3::Y,
                        ),
                    );
                }

                if world_voxel.y > 0 && !world.is_solid(world_voxel - IVec3::Y) {
                    bottom.push(
                        [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                        [0.0, -1.0, 0.0],
                        cell.texture_rotation,
                        grass_tint,
                        BOTTOM_SHADE,
                        skylight_at(
                            world,
                            &mut sky_heights,
                            max_loaded_chunk_y,
                            world_voxel - IVec3::Y,
                        ),
                    );
                }

                if !world.is_solid(world_voxel + IVec3::Z) {
                    front.push(
                        [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                        [0.0, SIDE_NORMAL_UP, SIDE_NORMAL_HORIZONTAL],
                        TextureRotation::default(),
                        grass_tint,
                        SOUTH_SHADE,
                        skylight_at(
                            world,
                            &mut sky_heights,
                            max_loaded_chunk_y,
                            world_voxel + IVec3::Z,
                        ),
                    );
                }

                if !world.is_solid(world_voxel - IVec3::Z) {
                    back.push(
                        [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
                        [0.0, SIDE_NORMAL_UP, -SIDE_NORMAL_HORIZONTAL],
                        TextureRotation::default(),
                        grass_tint,
                        NORTH_SHADE,
                        skylight_at(
                            world,
                            &mut sky_heights,
                            max_loaded_chunk_y,
                            world_voxel - IVec3::Z,
                        ),
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

fn skylight_at(
    world: &VoxelWorld,
    sky_heights: &mut HashMap<IVec2, i32>,
    max_loaded_chunk_y: i32,
    air_cell: IVec3,
) -> f32 {
    if open_to_sky(world, sky_heights, max_loaded_chunk_y, air_cell) {
        return 1.0;
    }

    for distance in 1..=SKY_LIGHT_SEARCH_RADIUS {
        for direction in SKY_LIGHT_DIRECTIONS {
            let sample = air_cell + direction * distance;

            if !world.is_loaded_at(sample) || world.is_solid(sample) {
                continue;
            }

            if open_to_sky(world, sky_heights, max_loaded_chunk_y, sample) {
                return (1.0 - distance as f32 * SKY_LIGHT_LATERAL_ATTENUATION)
                    .max(ENCLOSED_SKY_LIGHT);
            }
        }
    }

    ENCLOSED_SKY_LIGHT
}

fn open_to_sky(
    world: &VoxelWorld,
    sky_heights: &mut HashMap<IVec2, i32>,
    max_loaded_chunk_y: i32,
    air_cell: IVec3,
) -> bool {
    let column = IVec2::new(air_cell.x, air_cell.z);
    let highest_solid = *sky_heights.entry(column).or_insert_with(|| {
        world
            .highest_solid_y_in_column(column.x, column.y, max_loaded_chunk_y)
            .unwrap_or(-1)
    });

    air_cell.y > highest_solid
}
