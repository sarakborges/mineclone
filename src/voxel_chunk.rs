use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

pub const CHUNK_SIZE: usize = 16;
const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;
const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIZE;
const COLLISION_EPSILON: f32 = 0.0001;
const TEST_BLOCK_ID: &str = "mineclone:test_block";

#[derive(Component)]
pub struct VoxelChunk {
    blocks: [bool; CHUNK_VOLUME],
}

impl VoxelChunk {
    pub fn collision_test() -> Self {
        let mut chunk = Self {
            blocks: [false; CHUNK_VOLUME],
        };

        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set_solid(x, 0, z, true);
            }
        }

        for z in 3..13 {
            for y in 1..4 {
                chunk.set_solid(4, y, z, true);
            }
        }

        for x in 8..12 {
            for z in 4..7 {
                chunk.set_solid(x, 3, z, true);
            }
        }

        for x in 10..13 {
            for z in 10..13 {
                chunk.set_solid(x, 1, z, true);
            }
        }

        chunk
    }

    pub fn build_mesh(&self) -> Mesh {
        let mut positions = Vec::<[f32; 3]>::new();
        let mut normals = Vec::<[f32; 3]>::new();
        let mut indices = Vec::<u32>::new();

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if !self.is_solid(x as i32, y as i32, z as i32) {
                        continue;
                    }

                    let x0 = x as f32;
                    let y0 = y as f32;
                    let z0 = z as f32;
                    let x1 = x0 + 1.0;
                    let y1 = y0 + 1.0;
                    let z1 = z0 + 1.0;

                    if !self.is_solid(x as i32 + 1, y as i32, z as i32) {
                        push_face(
                            &mut positions,
                            &mut normals,
                            &mut indices,
                            [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]],
                            [1.0, 0.0, 0.0],
                        );
                    }

                    if !self.is_solid(x as i32 - 1, y as i32, z as i32) {
                        push_face(
                            &mut positions,
                            &mut normals,
                            &mut indices,
                            [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                            [-1.0, 0.0, 0.0],
                        );
                    }

                    if !self.is_solid(x as i32, y as i32 + 1, z as i32) {
                        push_face(
                            &mut positions,
                            &mut normals,
                            &mut indices,
                            [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
                            [0.0, 1.0, 0.0],
                        );
                    }

                    if !self.is_solid(x as i32, y as i32 - 1, z as i32) {
                        push_face(
                            &mut positions,
                            &mut normals,
                            &mut indices,
                            [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                            [0.0, -1.0, 0.0],
                        );
                    }

                    if !self.is_solid(x as i32, y as i32, z as i32 + 1) {
                        push_face(
                            &mut positions,
                            &mut normals,
                            &mut indices,
                            [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                            [0.0, 0.0, 1.0],
                        );
                    }

                    if !self.is_solid(x as i32, y as i32, z as i32 - 1) {
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

    pub fn collides_aabb(&self, min: Vec3, max: Vec3) -> bool {
        let chunk_size = CHUNK_SIZE as f32;

        if max.x <= 0.0
            || max.y <= 0.0
            || max.z <= 0.0
            || min.x >= chunk_size
            || min.y >= chunk_size
            || min.z >= chunk_size
        {
            return false;
        }

        let min_x = min.x.floor().max(0.0) as i32;
        let min_y = min.y.floor().max(0.0) as i32;
        let min_z = min.z.floor().max(0.0) as i32;
        let max_x = (max.x - COLLISION_EPSILON)
            .floor()
            .min((CHUNK_SIZE - 1) as f32) as i32;
        let max_y = (max.y - COLLISION_EPSILON)
            .floor()
            .min((CHUNK_SIZE - 1) as f32) as i32;
        let max_z = (max.z - COLLISION_EPSILON)
            .floor()
            .min((CHUNK_SIZE - 1) as f32) as i32;

        for y in min_y..=max_y {
            for z in min_z..=max_z {
                for x in min_x..=max_x {
                    if self.is_solid(x, y, z) {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn block_id_at(&self, x: i32, y: i32, z: i32) -> Option<&'static str> {
        self.is_solid(x, y, z).then_some(TEST_BLOCK_ID)
    }

    fn set_solid(&mut self, x: usize, y: usize, z: usize, solid: bool) {
        self.blocks[index(x, y, z)] = solid;
    }

    fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        if x < 0
            || y < 0
            || z < 0
            || x >= CHUNK_SIZE as i32
            || y >= CHUNK_SIZE as i32
            || z >= CHUNK_SIZE as i32
        {
            return false;
        }

        self.blocks[index(x as usize, y as usize, z as usize)]
    }
}

fn index(x: usize, y: usize, z: usize) -> usize {
    x + z * CHUNK_SIZE + y * CHUNK_AREA
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
