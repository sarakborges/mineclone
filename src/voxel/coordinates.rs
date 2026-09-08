use bevy::prelude::*;

use super::chunk::CHUNK_SIZE;

#[derive(Clone, Copy)]
pub struct ChunkCoordinates {
    pub chunk: IVec3,
}

pub fn split_dimension_position(position: Vec3) -> ChunkCoordinates {
    let chunk_size = CHUNK_SIZE as f32;
    let chunk = IVec3::new(
        (position.x / chunk_size).floor() as i32,
        (position.y / chunk_size).floor() as i32,
        (position.z / chunk_size).floor() as i32,
    );

    ChunkCoordinates { chunk }
}
