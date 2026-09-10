use crate::content::block_orientation::BlockOrientation;

use super::texture_rotation::TextureRotation;

#[derive(Clone, Copy)]
pub struct VoxelCell {
    pub block_id: &'static str,
    pub texture_rotation: TextureRotation,
    pub orientation: BlockOrientation,
}

impl VoxelCell {
    pub fn new(block_id: &'static str, texture_rotation: TextureRotation) -> Self {
        Self::oriented(block_id, texture_rotation, BlockOrientation::default())
    }

    pub fn oriented(
        block_id: &'static str,
        texture_rotation: TextureRotation,
        orientation: BlockOrientation,
    ) -> Self {
        Self {
            block_id,
            texture_rotation,
            orientation,
        }
    }
}
