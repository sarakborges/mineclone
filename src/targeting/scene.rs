use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
    voxel::{raycast::VoxelHit, world::VoxelWorld},
};

use super::block::TargetedBlock;

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

    pub(crate) fn world(&self) -> &VoxelWorld {
        &self.world
    }

    pub(crate) fn player_translation(&self) -> Vec3 {
        self.player.translation
    }
}
