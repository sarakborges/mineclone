use crate::voxel::chunk::CHUNK_SIZE;

#[cfg(test)]
use crate::voxel::chunk::CHUNK_VOLUME;

pub(super) fn column_index(x: usize, z: usize) -> usize {
    x + z * CHUNK_SIZE
}

pub(super) fn voxel_index(x: usize, y: usize, z: usize) -> usize {
    x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voxel_index_covers_the_chunk_without_collisions() {
        let mut seen = vec![false; CHUNK_VOLUME];

        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let index = voxel_index(x, y, z);
                    assert!(!seen[index]);
                    seen[index] = true;
                }
            }
        }

        assert!(seen.into_iter().all(|value| value));
    }
}
