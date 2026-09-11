mod animation;
mod model;

use bevy::prelude::*;

use crate::app::game_state::GameState;

pub(crate) use animation::ViewModelAnimation;
use animation::{ViewModelItemSwitch, advance_item_switch, animate_viewmodel};
use model::{setup_viewmodel_arm_assets, spawn_viewmodel, sync_held_block};

pub struct PlayerViewModelPlugin;

impl Plugin for PlayerViewModelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewModelAnimation>()
            .init_resource::<ViewModelItemSwitch>()
            .add_systems(Startup, setup_viewmodel_arm_assets)
            .add_systems(
                Update,
                (
                    spawn_viewmodel,
                    advance_item_switch,
                    sync_held_block,
                    animate_viewmodel,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}
