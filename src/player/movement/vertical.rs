use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    voxel::world::VoxelWorld,
    world::{game_rules::GameRules, tick::WorldTickClock},
};

#[derive(SystemParam)]
pub(super) struct VerticalMovementContext<'w> {
    pub(super) game_rules: Res<'w, GameRules>,
    pub(super) world_ticks: Res<'w, WorldTickClock>,
    pub(super) keys: Res<'w, ButtonInput<KeyCode>>,
    pub(super) world: Res<'w, VoxelWorld>,
}

impl VerticalMovementContext<'_> {
    pub(super) fn delta_seconds(&self) -> f32 {
        self.world_ticks.delta_seconds(&self.game_rules)
    }
}
