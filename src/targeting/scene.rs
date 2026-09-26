use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::builtin_ids::BIOME_TINT_METADATA_KEY,
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
    voxel::{raycast::VoxelHit, world::VoxelWorld},
};

use super::block::TargetedBlock;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct BlockTargetingVisualSnapshot {
    hit: Option<VoxelHit>,
    selected_slot: usize,
    selected_item: Option<&'static str>,
    player_translation: Vec3,
    block_content_revision: u64,
}

#[derive(SystemParam)]
pub(crate) struct BlockTargetingScene<'w, 's> {
    targeted: Res<'w, TargetedBlock>,
    hotbar: Res<'w, PlayerHotbar>,
    world: Res<'w, VoxelWorld>,
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
}

impl BlockTargetingScene<'_, '_> {
    pub(crate) fn hit(&self) -> Option<VoxelHit> {
        self.targeted.0
    }

    pub(crate) fn selected_slot(&self) -> usize {
        self.hotbar.selected_slot()
    }

    pub(crate) fn selected_item(&self) -> Option<&'static str> {
        self.hotbar.item_at(self.hotbar.selected_slot())
    }

    pub(crate) fn selected_biome_tint(&self) -> Option<&str> {
        self.hotbar
            .stack_at(self.hotbar.selected_slot())
            .and_then(|stack| stack.metadata().get(BIOME_TINT_METADATA_KEY))
    }

    pub(crate) fn selected_stack_changed(&self) -> bool {
        self.hotbar.is_changed()
    }

    pub(crate) fn world(&self) -> &VoxelWorld {
        &self.world
    }

    pub(crate) fn player_translation(&self) -> Vec3 {
        self.player.translation
    }

    pub(crate) fn player_forward(&self) -> Vec3 {
        self.player.forward().as_vec3()
    }

    pub(crate) fn visual_snapshot(&self) -> BlockTargetingVisualSnapshot {
        let selected_slot = self.selected_slot();
        BlockTargetingVisualSnapshot {
            hit: self.hit(),
            selected_slot,
            selected_item: self.hotbar.item_at(selected_slot),
            player_translation: self.player_translation(),
            block_content_revision: self.world.block_content_revision(),
        }
    }
}
