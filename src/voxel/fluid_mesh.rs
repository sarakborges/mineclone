use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use crate::content::fluid::FluidId;

use super::{
    chunk::{CHUNK_SIZE, VoxelChunk},
    mesh::{
        BlockFace,
        lighting::{FaceLighting, face_lighting, should_flip_diagonal},
    },
    world::VoxelWorld,
};

const FACE_UVS: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

pub struct ChunkFluidMesh {
    pub fluid_id: FluidId,
    pub mesh: Mesh,
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
    fn push(&mut self, vertices: [[f32; 3]; 4], normal: [f32; 3], lighting: FaceLighting) {
        let base = self.positions.len() as u32;
        let vertex_colors = lighting.ambient_occlusion.map(|ao| [1.0, 1.0, 1.0, ao]);

        self.positions.extend(vertices);
        self.normals.extend([normal; 4]);
        self.uvs.extend(FACE_UVS);
        self.light_uvs.extend(lighting.channels);
        self.colors.extend(vertex_colors);

        if should_flip_diagonal(lighting.ambient_occlusion) {
            self.indices
                .extend([base, base + 1, base + 3, base + 1, base + 2, base + 3]);
        } else {
            self.indices
                .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
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

pub fn build_fluid_meshes(
    world: &VoxelWorld,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
) -> Vec<ChunkFluidMesh> {
    let mut buffers = HashMap::<FluidId, MeshBuffers>::new();
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_origin = chunk_coord * chunk_size;

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let Some(cell) = chunk.fluid_at(x as i32, y as i32, z as i32) else {
                    continue;
                };

                let fluid = buffers.entry(cell.fluid_id).or_default();
                let local = IVec3::new(x as i32, y as i32, z as i32);
                let world_voxel = chunk_origin + local;
                let x0 = x as f32;
                let y0 = y as f32;
                let z0 = z as f32;
                let x1 = x0 + 1.0;
                let z1 = z0 + 1.0;
                let h00 = fluid_corner_height(world, world_voxel, cell.fluid_id, -1, -1);
                let h10 = fluid_corner_height(world, world_voxel, cell.fluid_id, 1, -1);
                let h11 = fluid_corner_height(world, world_voxel, cell.fluid_id, 1, 1);
                let h01 = fluid_corner_height(world, world_voxel, cell.fluid_id, -1, 1);

                if face_is_exposed(world, world_voxel + IVec3::X, cell.fluid_id) {
                    fluid.push(
                        [
                            [x1, y0, z1],
                            [x1, y0, z0],
                            [x1, y0 + h10, z0],
                            [x1, y0 + h11, z1],
                        ],
                        [1.0, 0.0, 0.0],
                        face_lighting(world, world_voxel, BlockFace::Right),
                    );
                }

                if face_is_exposed(world, world_voxel - IVec3::X, cell.fluid_id) {
                    fluid.push(
                        [
                            [x0, y0, z0],
                            [x0, y0, z1],
                            [x0, y0 + h01, z1],
                            [x0, y0 + h00, z0],
                        ],
                        [-1.0, 0.0, 0.0],
                        face_lighting(world, world_voxel, BlockFace::Left),
                    );
                }

                if face_is_exposed(world, world_voxel + IVec3::Y, cell.fluid_id) {
                    fluid.push(
                        [
                            [x0, y0 + h01, z1],
                            [x1, y0 + h11, z1],
                            [x1, y0 + h10, z0],
                            [x0, y0 + h00, z0],
                        ],
                        [0.0, 1.0, 0.0],
                        face_lighting(world, world_voxel, BlockFace::Top),
                    );
                }

                if world_voxel.y > 0
                    && face_is_exposed(world, world_voxel - IVec3::Y, cell.fluid_id)
                {
                    fluid.push(
                        [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                        [0.0, -1.0, 0.0],
                        face_lighting(world, world_voxel, BlockFace::Bottom),
                    );
                }

                if face_is_exposed(world, world_voxel + IVec3::Z, cell.fluid_id) {
                    fluid.push(
                        [
                            [x0, y0, z1],
                            [x1, y0, z1],
                            [x1, y0 + h11, z1],
                            [x0, y0 + h01, z1],
                        ],
                        [0.0, 0.0, 1.0],
                        face_lighting(world, world_voxel, BlockFace::Front),
                    );
                }

                if face_is_exposed(world, world_voxel - IVec3::Z, cell.fluid_id) {
                    fluid.push(
                        [
                            [x1, y0, z0],
                            [x0, y0, z0],
                            [x0, y0 + h00, z0],
                            [x1, y0 + h10, z0],
                        ],
                        [0.0, 0.0, -1.0],
                        face_lighting(world, world_voxel, BlockFace::Back),
                    );
                }
            }
        }
    }

    buffers
        .into_iter()
        .filter_map(|(fluid_id, buffers)| {
            buffers
                .into_mesh()
                .map(|mesh| ChunkFluidMesh { fluid_id, mesh })
        })
        .collect()
}

fn fluid_corner_height(
    world: &VoxelWorld,
    position: IVec3,
    fluid_id: FluidId,
    x_sign: i32,
    z_sign: i32,
) -> f32 {
    let offsets = [
        IVec3::ZERO,
        IVec3::new(x_sign, 0, 0),
        IVec3::new(0, 0, z_sign),
        IVec3::new(x_sign, 0, z_sign),
    ];
    let mut total = 0.0;
    let mut count = 0.0;

    for offset in offsets {
        let sample_position = position + offset;
        if world
            .fluid_at(sample_position + IVec3::Y)
            .is_some_and(|cell| cell.fluid_id == fluid_id)
        {
            return 1.0;
        }

        if let Some(cell) = world
            .fluid_at(sample_position)
            .filter(|cell| cell.fluid_id == fluid_id)
        {
            total += cell.height();
            count += 1.0;
        }
    }

    if count > 0.0 {
        total / count
    } else {
        0.0
    }
}

fn face_is_exposed(world: &VoxelWorld, position: IVec3, fluid_id: FluidId) -> bool {
    if world.is_solid(position) {
        return false;
    }

    match world.fluid_at(position) {
        Some(neighbor) => neighbor.fluid_id != fluid_id,
        None => true,
    }
}
