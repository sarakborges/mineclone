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

pub fn build_chunk_mesh<F>(
    world: &VoxelWorld,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    tint_at: F,
) -> Mesh
where
    F: Fn(IVec3) -> [f32; 3],
{
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut colors = Vec::<[f32; 4]>::new();
    let mut indices = Vec::<u32>::new();
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
                let tint = tint_at(world_voxel);
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
                        &mut uvs,
                        &mut colors,
                        &mut indices,
                        [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]],
                        [SIDE_NORMAL_HORIZONTAL, SIDE_NORMAL_UP, 0.0],
                        cell.texture_rotation,
                        tint,
                        EAST_SHADE,
                    );
                }

                if !world.is_solid(world_voxel - IVec3::X) {
                    push_face(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut colors,
                        &mut indices,
                        [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                        [-SIDE_NORMAL_HORIZONTAL, SIDE_NORMAL_UP, 0.0],
                        cell.texture_rotation,
                        tint,
                        WEST_SHADE,
                    );
                }

                if !world.is_solid(world_voxel + IVec3::Y) {
                    push_face(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut colors,
                        &mut indices,
                        [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
                        [0.0, 1.0, 0.0],
                        cell.texture_rotation,
                        tint,
                        TOP_SHADE,
                    );
                }

                if world_voxel.y > 0 && !world.is_solid(world_voxel - IVec3::Y) {
                    push_face(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut colors,
                        &mut indices,
                        [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                        [0.0, -1.0, 0.0],
                        cell.texture_rotation,
                        tint,
                        BOTTOM_SHADE,
                    );
                }

                if !world.is_solid(world_voxel + IVec3::Z) {
                    push_face(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut colors,
                        &mut indices,
                        [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                        [0.0, SIDE_NORMAL_UP, SIDE_NORMAL_HORIZONTAL],
                        cell.texture_rotation,
                        tint,
                        SOUTH_SHADE,
                    );
                }

                if !world.is_solid(world_voxel - IVec3::Z) {
                    push_face(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut colors,
                        &mut indices,
                        [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
                        [0.0, SIDE_NORMAL_UP, -SIDE_NORMAL_HORIZONTAL],
                        cell.texture_rotation,
                        tint,
                        NORTH_SHADE,
                    );
                }
            }
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}
