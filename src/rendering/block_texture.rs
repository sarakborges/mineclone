use std::collections::{BTreeSet, HashMap};

use bevy::prelude::*;

use crate::{
    content::block::{BlockDefinition, BlockRegistry, BlockTextureLayer},
    voxel::block_face::BlockFace,
};

pub(crate) const TERRAIN_SHARED_MASK_CUTOFF: f32 = 0.5;

pub(crate) fn terrain_array_alpha_signature(
    definition: &BlockDefinition,
) -> Option<(bool, Option<u32>)> {
    if definition.alpha_blend {
        return Some((true, None));
    }

    if definition
        .alpha_cutoff
        .is_none_or(|cutoff| cutoff.to_bits() == TERRAIN_SHARED_MASK_CUTOFF.to_bits())
    {
        return Some((
            false,
            Some(TERRAIN_SHARED_MASK_CUTOFF.to_bits()),
        ));
    }

    None
}

const TERRAIN_TEXTURE_INDEX_BITS: u32 = 10;
const TERRAIN_TEXTURE_INDEX_MASK: u32 = (1 << TERRAIN_TEXTURE_INDEX_BITS) - 1;
const TERRAIN_TEXTURE_NONE_INDEX: u32 = TERRAIN_TEXTURE_INDEX_MASK;
const TERRAIN_TEXTURE_FLAG_SHIFT: u32 = TERRAIN_TEXTURE_INDEX_BITS * 2;
const TERRAIN_TEXTURE_BASE_DYABLE: u32 = 1;
const TERRAIN_TEXTURE_OVERLAY_DYABLE: u32 = 2;
const MAX_TERRAIN_TEXTURES: usize = TERRAIN_TEXTURE_NONE_INDEX as usize - 1;

#[derive(Clone, Default)]
pub(crate) struct TerrainTextureTable {
    paths: Vec<String>,
    indices: HashMap<String, u16>,
}

impl TerrainTextureTable {
    pub(crate) fn from_blocks(blocks: &BlockRegistry) -> Self {
        let mut unique = BTreeSet::<String>::new();
        for block in blocks.iter() {
            for face in BlockFace::ALL {
                let layers = block_face_texture_layers(face, block);
                if layers.len() > 2 {
                    continue;
                }
                for layer in layers {
                    unique.insert(layer.texture.clone());
                }
            }
        }

        assert!(
            unique.len() <= MAX_TERRAIN_TEXTURES,
            "terrain texture array supports at most {MAX_TERRAIN_TEXTURES} unique block textures"
        );

        let paths = unique.into_iter().collect::<Vec<_>>();
        let indices = paths
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let array_index = u16::try_from(index + 1)
                    .expect("terrain texture array index must fit in u16");
                (path.clone(), array_index)
            })
            .collect();

        Self { paths, indices }
    }

    pub(crate) fn paths(&self) -> &[String] {
        &self.paths
    }

    pub(crate) fn layer_count(&self) -> u32 {
        u32::try_from(self.paths.len() + 1)
            .expect("terrain texture array layer count must fit in u32")
    }

    pub(crate) fn encoded_layers(&self, layers: &[BlockTextureLayer]) -> Option<f32> {
        if layers.len() > 2 {
            return None;
        }

        let base = layers.first().map_or(0_u32, |layer| {
            u32::from(*self.indices.get(&layer.texture).unwrap_or_else(|| {
                panic!("missing terrain texture index for {}", layer.texture)
            }))
        });
        let overlay = layers.get(1).map_or(TERRAIN_TEXTURE_NONE_INDEX, |layer| {
            u32::from(*self.indices.get(&layer.texture).unwrap_or_else(|| {
                panic!("missing terrain texture index for {}", layer.texture)
            }))
        });
        let mut flags = 0_u32;
        if layers.first().is_some_and(|layer| layer.dyable) {
            flags |= TERRAIN_TEXTURE_BASE_DYABLE;
        }
        if layers.get(1).is_some_and(|layer| layer.dyable) {
            flags |= TERRAIN_TEXTURE_OVERLAY_DYABLE;
        }

        let encoded = base
            | (overlay << TERRAIN_TEXTURE_INDEX_BITS)
            | (flags << TERRAIN_TEXTURE_FLAG_SHIFT);
        debug_assert!(encoded <= 0x00ff_ffff);
        Some(encoded as f32)
    }
}

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
