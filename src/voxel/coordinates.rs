use bevy::prelude::*;

use super::chunk::CHUNK_SIZE;

#[derive(Clone, Copy)]
pub struct ChunkCoordinates {
    pub chunk: IVec3,
}

pub fn split_dimension_position(position: Vec3) -> ChunkCoordinates {
    ChunkCoordinates {
        chunk: chunk_coord_from_position(position),
    }
}

pub(crate) fn chunk_coord_from_world(world_position: IVec3) -> IVec3 {
    let chunk_size = CHUNK_SIZE as i32;
    IVec3::new(
        world_position.x.div_euclid(chunk_size),
        world_position.y.div_euclid(chunk_size),
        world_position.z.div_euclid(chunk_size),
    )
}

pub(crate) fn split_world_position(world_position: IVec3) -> (IVec3, IVec3) {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk = chunk_coord_from_world(world_position);
    let local = IVec3::new(
        world_position.x.rem_euclid(chunk_size),
        world_position.y.rem_euclid(chunk_size),
        world_position.z.rem_euclid(chunk_size),
    );

    (chunk, local)
}

pub(crate) fn chunk_origin(coord: IVec3) -> IVec3 {
    coord * CHUNK_SIZE as i32
}

pub(crate) fn chunks_for_block_extent(extent: i32) -> i32 {
    if extent <= 0 {
        return 0;
    }

    let chunk_size = CHUNK_SIZE as i32;
    extent.saturating_add(chunk_size - 1) / chunk_size
}

fn chunk_coord_from_position(position: Vec3) -> IVec3 {
    let chunk_size = CHUNK_SIZE as f32;
    IVec3::new(
        (position.x / chunk_size).floor() as i32,
        (position.y / chunk_size).floor() as i32,
        (position.z / chunk_size).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_world_position_splits_with_euclidean_coordinates() {
        let world = IVec3::new(-1, 17, -17);
        let (chunk, local) = split_world_position(world);

        assert_eq!(chunk, IVec3::new(-1, 1, -2));
        assert_eq!(local, IVec3::new(15, 1, 15));
        assert_eq!(chunk_origin(chunk) + local, world);
    }

    #[test]
    fn block_extent_rounds_up_to_whole_chunks() {
        assert_eq!(chunks_for_block_extent(0), 0);
        assert_eq!(chunks_for_block_extent(1), 1);
        assert_eq!(chunks_for_block_extent(CHUNK_SIZE as i32), 1);
        assert_eq!(chunks_for_block_extent(CHUNK_SIZE as i32 + 1), 2);
    }
}
