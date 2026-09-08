use super::chunk::{VoxelChunk, CHUNK_SIZE};

pub fn collision_test_chunk() -> VoxelChunk {
    let mut chunk = VoxelChunk::empty();

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
