use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{biome::BiomeRegistry, block::BlockRegistry},
    world::biome_field::BiomeField,
};

use super::block_tint::block_tint_at;

#[derive(SystemParam)]
pub(crate) struct BlockVisualContent<'w> {
    pub(crate) asset_server: Res<'w, AssetServer>,
    pub(crate) blocks: Res<'w, BlockRegistry>,
    pub(crate) biomes: Res<'w, BiomeRegistry>,
    pub(crate) biome_field: Res<'w, BiomeField>,
}

impl BlockVisualContent<'_> {
    pub(crate) fn block_definitions_changed(&self) -> bool {
        self.blocks.is_changed()
    }

    pub(crate) fn inputs_changed(&self) -> bool {
        self.block_definitions_changed()
            || self.biomes.is_changed()
            || self.biome_field.is_changed()
    }

    pub(crate) fn tint_at(&self, block_id: &str, position: Vec2) -> Option<Color> {
        let block = self.blocks.get(block_id)?;
        Some(block_tint_at(
            block.tint,
            position,
            &self.biome_field,
            &self.biomes,
        ))
    }
}
