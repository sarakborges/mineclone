use bevy::{ecs::system::SystemParam, prelude::*};

use crate::content::block::BlockRegistry;

use super::block_tint::block_tint;

#[derive(SystemParam)]
pub(crate) struct BlockVisualContent<'w> {
    pub(crate) asset_server: Res<'w, AssetServer>,
    pub(crate) blocks: Res<'w, BlockRegistry>,
}

impl BlockVisualContent<'_> {
    pub(crate) fn block_definitions_changed(&self) -> bool {
        self.blocks.is_changed()
    }

    pub(crate) fn inputs_changed(&self) -> bool {
        self.block_definitions_changed()
    }

    pub(crate) fn tint_at(&self, block_id: &str, _position: Vec2) -> Option<Color> {
        let block = self.blocks.get(block_id)?;
        Some(block_tint(block.tint))
    }

    pub(crate) fn tint_at_with_override(
        &self,
        block_id: &str,
        _position: Vec2,
        _biome_override: Option<&str>,
    ) -> Option<Color> {
        let block = self.blocks.get(block_id)?;
        Some(block_tint(block.tint))
    }
}
