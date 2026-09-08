use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

use super::{
    chunk::{VoxelChunk, CHUNK_SIZE},
    world::VoxelWorld,
};

pub fn build_chunk_mesh(world: &VoxelWorld, chunk_coord: IVec2, chunk: &VoxelChunk) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut indices = Vec::<u32>::new();
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_origin = IVec3::new(chunk_coord.x * chunk_size, 0, chunk_coord.y * chunk_size);

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                if !chunk.is_solid(x as i32, y as i32, z as i32) {
                    continue;
                }

                let local = IVec3::new(x as i32, y as i32, z as i32);
                let world_voxel = chunk_origin + local;
                let x0 = x as f32;
                let y0 = y as f32;
                let z0 = z as f32;
                let x1 = x0 + 1.0;
                let y1 = y0 + 1.0;
                let z1 = z0 + 1.0;

                if !world.is_solid(world_voxel + IVec3::X) {
                    push_face(
                        &mut positions,
                        &mut normals,
                        &mut indices,
                        [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]],
                        [1.0, 0.0, 0.0],
                    );
                }

                if !world.is_solid(world_voxel - IVec3::X) {
                    push_face(
                        &mut positions,
                        &mut normals,
                        &mut indices,
                        [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                        [-1.0, 0.0, 0.0],
                    );
                }

                if !world.is_solid(world_voxel + IVec3::Y) {
                    push_face(
                        &mut positions,
                        &mut normals,
                        &mut indices,
                        [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
                        [0.0, 1.0, 0.0],
                    );
                }

                if !world.is_solid(world_voxel - IVec3::Y) {
                    push_face(
                        &mut positions,
                        &mut normals,
                        &mut indices,
                        [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                        [0.0, -1.0, 0.0],
                    );
                }

                if !world.is_solid(world_voxel + IVec3::Z) {
                    push_face(
                        &mut positions,
                        &mut normals,
                        &mut indices,
                        [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                        [0.0, 0.0, 1.0],
                    );
                }

                if !world.is_solid(world_voxel - IVec3::Z) {
                    push_face(
                        &mut positions,
                        &mut normals,
                        &mut indices,
                        [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
                        [0.0, 0.0, -1.0],
                    );
                }
            }
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

fn push_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    vertices: [[f32; 3]; 4],
    normal: [f32; 3],
) {
    let start = positions.len() as u32;
    positions.extend(vertices);
    normals.extend([normal; 4]);
    indices.extend([start, start + 1, start + 2, start, start + 2, start + 3]);
}
