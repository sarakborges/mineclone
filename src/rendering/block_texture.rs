use bevy::prelude::*;

use crate::{
    content::block::{BlockDefinition, BlockTextureLayer},
    voxel::block_face::BlockFace,
};

pub(crate) fn block_face_texture_layers(
    face: BlockFace,
    block: &BlockDefinition,
) -> &[BlockTextureLayer] {
    let layers = match face {
        BlockFace::Right => block.textures.right.as_slice(),
        BlockFace::Left => block.textures.left.as_slice(),
        BlockFace::Top => block.textures.top.as_slice(),
        BlockFace::Bottom => block.textures.bottom.as_slice(),
        BlockFace::Front => block.textures.front.as_slice(),
        BlockFace::Back => block.textures.back.as_slice(),
    };

    if layers.is_empty() {
        first_block_texture_layers(block)
    } else {
        layers
    }
}

pub(crate) fn block_face_material_face(face: BlockFace, block: &BlockDefinition) -> BlockFace {
    let layers = block_face_texture_layers(face, block);

    BlockFace::ALL
        .into_iter()
        .find(|candidate| same_texture_layers(block_face_texture_layers(*candidate, block), layers))
        .unwrap_or(face)
}

pub(crate) fn load_block_texture_layer(
    asset_server: &AssetServer,
    layer: &BlockTextureLayer,
) -> Handle<Image> {
    asset_server.load(layer.texture.clone())
}

fn same_texture_layers(left: &[BlockTextureLayer], right: &[BlockTextureLayer]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.texture == right.texture && left.dyable == right.dyable)
}

fn first_block_texture_layers(block: &BlockDefinition) -> &[BlockTextureLayer] {
    [
        block.textures.top.as_slice(),
        block.textures.front.as_slice(),
        block.textures.right.as_slice(),
        block.textures.left.as_slice(),
        block.textures.back.as_slice(),
        block.textures.bottom.as_slice(),
    ]
    .into_iter()
    .find(|layers| !layers.is_empty())
    .unwrap_or_default()
}
