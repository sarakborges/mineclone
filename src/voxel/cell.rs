use super::texture_rotation::TextureRotation;

#[derive(Clone, Copy)]
pub struct VoxelCell {
    pub block_id: &'static str,
    pub texture_rotation: TextureRotation,
}

impl VoxelCell {
    pub fn new(block_id: &'static str, texture_rotation: TextureRotation) -> Self {
        Self {
            block_id,
            texture_rotation,
        }
    }
}
