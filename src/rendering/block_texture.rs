use crate::{content::block::BlockDefinition, voxel::block_face::BlockFace};

pub(crate) fn block_face_texture(face: BlockFace, block: &BlockDefinition) -> Option<&str> {
    let texture = match face {
        BlockFace::Right => block.textures.right.as_str(),
        BlockFace::Left => block.textures.left.as_str(),
        BlockFace::Top => block.textures.top.as_str(),
        BlockFace::Bottom => block.textures.bottom.as_str(),
        BlockFace::Front => block.textures.front.as_str(),
        BlockFace::Back => block.textures.back.as_str(),
    };

    if !texture.is_empty() {
        Some(texture)
    } else {
        first_block_texture(block)
    }
}

fn first_block_texture(block: &BlockDefinition) -> Option<&str> {
    [
        block.textures.top.as_str(),
        block.textures.front.as_str(),
        block.textures.right.as_str(),
        block.textures.left.as_str(),
        block.textures.back.as_str(),
        block.textures.bottom.as_str(),
    ]
    .into_iter()
    .find(|texture| !texture.is_empty())
}
