mod face;

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
const BOTTOM_SHADE: f32 = 0.82;
const EAST_SHADE: f32 = 0.92;
const WEST_SHADE: f32 = 0.88;
const SOUTH_SHADE: f32 = 0.94;
const NORTH_SHADE: f32 = 0.90;

const LIGHT_BRIGHTNESS: [f32; 16] = [
    0.045, 0.060, 0.080, 0.105, 0.135, 0.175, 0.225, 0.285,
    0.355, 0.435, 0.525, 0.625, 0.735, 0.835, 0.955, 1.000,
];

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
        skylight: [f32; 4],
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
                        [1.0, 0.0, 0.0],
                        TextureRotation::default(),
                        grass_tint,
                        EAST_SHADE,
                        face_skylight(world, world_voxel, BlockFace::Right),
                    );
                }

                if !world.is_solid(world_voxel - IVec3::X) {
                    left.push(
                        [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                        [-1.0, 0.0, 0.0],
                        TextureRotation::default(),
                        grass_tint,
                        WEST_SHADE,
                        face_skylight(world, world_voxel, BlockFace::Left),
                    );
                }

                if !world.is_solid(world_voxel + IVec3::Y) {
                    top.push(
                        [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
                        [0.0, 1.0, 0.0],
                        cell.texture_rotation,
                        grass_tint,
                        TOP_SHADE,
                        face_skylight(world, world_voxel, BlockFace::Top),
                    );
                }

                if world_voxel.y > 0 && !world.is_solid(world_voxel - IVec3::Y) {
                    bottom.push(
                        [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                        [0.0, -1.0, 0.0],
                        cell.texture_rotation,
                        grass_tint,
                        BOTTOM_SHADE,
                        face_skylight(world, world_voxel, BlockFace::Bottom),
                    );
                }

                if !world.is_solid(world_voxel + IVec3::Z) {
                    front.push(
                        [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                        [0.0, 0.0, 1.0],
                        TextureRotation::default(),
                        grass_tint,
                        SOUTH_SHADE,
                        face_skylight(world, world_voxel, BlockFace::Front),
                    );
                }

                if !world.is_solid(world_voxel - IVec3::Z) {
                    back.push(
                        [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
                        [0.0, 0.0, -1.0],
                        TextureRotation::default(),
                        grass_tint,
                        NORTH_SHADE,
                        face_skylight(world, world_voxel, BlockFace::Back),
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

fn face_skylight(world: &VoxelWorld, voxel: IVec3, face: BlockFace) -> [f32; 4] {
    let (normal, tangent_a, tangent_b, signs) = match face {
        BlockFace::Right => (
            IVec3::X,
            IVec3::Y,
            IVec3::Z,
            [(-1, 1), (-1, -1), (1, -1), (1, 1)],
        ),
        BlockFace::Left => (
            IVec3::NEG_X,
            IVec3::Y,
            IVec3::Z,
            [(-1, -1), (-1, 1), (1, 1), (1, -1)],
        ),
        BlockFace::Top => (
            IVec3::Y,
            IVec3::X,
            IVec3::Z,
            [(-1, 1), (1, 1), (1, -1), (-1, -1)],
        ),
        BlockFace::Bottom => (
            IVec3::NEG_Y,
            IVec3::X,
            IVec3::Z,
            [(-1, -1), (1, -1), (1, 1), (-1, 1)],
        ),
        BlockFace::Front => (
            IVec3::Z,
            IVec3::X,
            IVec3::Y,
            [(-1, -1), (1, -1), (1, 1), (-1, 1)],
        ),
        BlockFace::Back => (
            IVec3::NEG_Z,
            IVec3::X,
            IVec3::Y,
            [(1, -1), (-1, -1), (-1, 1), (1, 1)],
        ),
    };
    let base = voxel + normal;

    signs.map(|(sign_a, sign_b)| {
        vertex_skylight(
            world,
            base,
            tangent_a * sign_a,
            tangent_b * sign_b,
        )
    })
}

fn vertex_skylight(
    world: &VoxelWorld,
    base: IVec3,
    offset_a: IVec3,
    offset_b: IVec3,
) -> f32 {
    let samples = [
        base,
        base + offset_a,
        base + offset_b,
        base + offset_a + offset_b,
    ];

    samples
        .into_iter()
        .map(|position| skylight_brightness(world, position))
        .sum::<f32>()
        / samples.len() as f32
}

fn skylight_brightness(world: &VoxelWorld, air_cell: IVec3) -> f32 {
    if air_cell.y < 0 {
        return LIGHT_BRIGHTNESS[0];
    }

    // Missing neighbor chunks are temporary while streaming. Treat their edge light as
    // open sky so chunk borders do not flash black before the neighbor is generated.
    if !world.is_loaded_at(air_cell) {
        return LIGHT_BRIGHTNESS[15];
    }

    LIGHT_BRIGHTNESS[world.skylight_at(air_cell).min(15) as usize]
}
