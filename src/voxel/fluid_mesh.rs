use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::fluid::FluidId;

use super::{
    block_face::BlockFace,
    chunk::{CHUNK_SIZE, VoxelChunk},
    mesh::lighting::{FaceLighting, face_lighting, should_flip_diagonal},
    mesh_buffer::VoxelMeshBuffer,
    quad::VOXEL_FACE_UVS,
    world::VoxelWorld,
};

pub struct ChunkFluidMesh {
    pub fluid_id: FluidId,
    pub mesh: Mesh,
}

pub fn build_fluid_meshes<F>(
    world: &VoxelWorld,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    tint_at: F,
) -> Vec<ChunkFluidMesh>
where
    F: Fn(IVec3, FluidId) -> [f32; 3],
{
    let mut buffers = HashMap::<FluidId, VoxelMeshBuffer>::new();
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
                let tint = tint_at(world_voxel, cell.fluid_id);
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
                    push_fluid_face(
                        fluid,
                        [
                            [x1, y0, z1],
                            [x1, y0, z0],
                            [x1, y0 + h10, z0],
                            [x1, y0 + h11, z1],
                        ],
                        [1.0, 0.0, 0.0],
                        face_lighting(world, world_voxel, BlockFace::Right),
                        tint,
                    );
                }

                if face_is_exposed(world, world_voxel - IVec3::X, cell.fluid_id) {
                    push_fluid_face(
                        fluid,
                        [
                            [x0, y0, z0],
                            [x0, y0, z1],
                            [x0, y0 + h01, z1],
                            [x0, y0 + h00, z0],
                        ],
                        [-1.0, 0.0, 0.0],
                        face_lighting(world, world_voxel, BlockFace::Left),
                        tint,
                    );
                }

                if face_is_exposed(world, world_voxel + IVec3::Y, cell.fluid_id) {
                    push_fluid_face(
                        fluid,
                        [
                            [x0, y0 + h01, z1],
                            [x1, y0 + h11, z1],
                            [x1, y0 + h10, z0],
                            [x0, y0 + h00, z0],
                        ],
                        [0.0, 1.0, 0.0],
                        face_lighting(world, world_voxel, BlockFace::Top),
                        tint,
                    );
                }

                if world_voxel.y > 0
                    && face_is_exposed(world, world_voxel - IVec3::Y, cell.fluid_id)
                {
                    push_fluid_face(
                        fluid,
                        [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                        [0.0, -1.0, 0.0],
                        face_lighting(world, world_voxel, BlockFace::Bottom),
                        tint,
                    );
                }

                if face_is_exposed(world, world_voxel + IVec3::Z, cell.fluid_id) {
                    push_fluid_face(
                        fluid,
                        [
                            [x0, y0, z1],
                            [x1, y0, z1],
                            [x1, y0 + h11, z1],
                            [x0, y0 + h01, z1],
                        ],
                        [0.0, 0.0, 1.0],
                        face_lighting(world, world_voxel, BlockFace::Front),
                        tint,
                    );
                }

                if face_is_exposed(world, world_voxel - IVec3::Z, cell.fluid_id) {
                    push_fluid_face(
                        fluid,
                        [
                            [x1, y0, z0],
                            [x0, y0, z0],
                            [x0, y0 + h00, z0],
                            [x1, y0 + h10, z0],
                        ],
                        [0.0, 0.0, -1.0],
                        face_lighting(world, world_voxel, BlockFace::Back),
                        tint,
                    );
                }
            }
        }
    }

    buffers
        .into_iter()
        .filter_map(|(fluid_id, buffer)| {
            buffer
                .into_mesh()
                .map(|mesh| ChunkFluidMesh { fluid_id, mesh })
        })
        .collect()
}

fn push_fluid_face(
    buffer: &mut VoxelMeshBuffer,
    vertices: [[f32; 3]; 4],
    normal: [f32; 3],
    lighting: FaceLighting,
    tint: [f32; 3],
) {
    let colors = lighting
        .ambient_occlusion
        .map(|ao| [tint[0], tint[1], tint[2], ao]);

    buffer.push_quad(
        vertices,
        normal,
        VOXEL_FACE_UVS,
        lighting.channels,
        colors,
        should_flip_diagonal(lighting.ambient_occlusion),
    );
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

    if count > 0.0 { total / count } else { 0.0 }
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
