use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::fluid::FluidId;

use super::{
    block_face::BlockFace,
    chunk::{CHUNK_SIZE, VoxelChunk},
    mesh_buffer::VoxelMeshBuffer,
    mesh_lighting::{face_lighting, push_lit_quad},
    quad::VOXEL_FACE_UVS,
    world::VoxelWorld,
};

pub struct ChunkFluidMesh {
    pub fluid_id: FluidId,
    pub mesh: Mesh,
}

#[derive(Clone, Copy)]
struct FluidFaceHeights {
    h00: f32,
    h10: f32,
    h11: f32,
    h01: f32,
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
                let world_voxel = chunk_origin + IVec3::new(x as i32, y as i32, z as i32);
                let tint = tint_at(world_voxel, cell.fluid_id);
                let heights = FluidFaceHeights {
                    h00: fluid_corner_height(world, world_voxel, cell.fluid_id, -1, -1),
                    h10: fluid_corner_height(world, world_voxel, cell.fluid_id, 1, -1),
                    h11: fluid_corner_height(world, world_voxel, cell.fluid_id, 1, 1),
                    h01: fluid_corner_height(world, world_voxel, cell.fluid_id, -1, 1),
                };

                for face in BlockFace::ALL {
                    if face == BlockFace::Bottom && world_voxel.y <= 0 {
                        continue;
                    }
                    if !face_is_exposed(world, world_voxel + face.offset(), cell.fluid_id) {
                        continue;
                    }

                    push_lit_quad(
                        fluid,
                        fluid_face_vertices(face, x as f32, y as f32, z as f32, heights),
                        face.normal(),
                        VOXEL_FACE_UVS,
                        tint,
                        face_lighting(world, world_voxel, face, false),
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

fn fluid_face_vertices(
    face: BlockFace,
    x0: f32,
    y0: f32,
    z0: f32,
    heights: FluidFaceHeights,
) -> [[f32; 3]; 4] {
    let x1 = x0 + 1.0;
    let z1 = z0 + 1.0;

    match face {
        BlockFace::Right => [
            [x1, y0, z1],
            [x1, y0, z0],
            [x1, y0 + heights.h10, z0],
            [x1, y0 + heights.h11, z1],
        ],
        BlockFace::Left => [
            [x0, y0, z0],
            [x0, y0, z1],
            [x0, y0 + heights.h01, z1],
            [x0, y0 + heights.h00, z0],
        ],
        BlockFace::Top => [
            [x0, y0 + heights.h01, z1],
            [x1, y0 + heights.h11, z1],
            [x1, y0 + heights.h10, z0],
            [x0, y0 + heights.h00, z0],
        ],
        BlockFace::Bottom => [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
        BlockFace::Front => [
            [x0, y0, z1],
            [x1, y0, z1],
            [x1, y0 + heights.h11, z1],
            [x0, y0 + heights.h01, z1],
        ],
        BlockFace::Back => [
            [x1, y0, z0],
            [x0, y0, z0],
            [x0, y0 + heights.h00, z0],
            [x1, y0 + heights.h10, z0],
        ],
    }
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
