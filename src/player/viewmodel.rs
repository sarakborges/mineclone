mod animation;
mod held_brush;
mod model;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    targeting::block::BlockTargetingSet,
    ui::visibility::set_visibility,
};

pub(crate) use animation::ViewModelAnimation;
use animation::{PlayerViewModel, ViewModelItemSwitch, advance_item_switch, animate_viewmodel};
use held_brush::{setup_held_brush_assets, spawn_held_brush, sync_held_brush};
use model::{setup_viewmodel_arm_assets, spawn_viewmodel, sync_held_block};

pub struct PlayerViewModelPlugin;

impl Plugin for PlayerViewModelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewModelAnimation>()
            .init_resource::<ViewModelItemSwitch>()
            .add_systems(Startup, setup_viewmodel_arm_assets)
            .add_systems(OnEnter(GameState::Gameplay), setup_held_brush_assets)
            .add_systems(
                OnEnter(PauseState::Paused),
                set_visibility::<PlayerViewModel, false>,
            )
            .add_systems(
                OnEnter(PauseState::Running),
                set_visibility::<PlayerViewModel, true>,
            )
            .add_systems(
                Update,
                (
                    spawn_viewmodel,
                    spawn_held_brush,
                    advance_item_switch,
                    sync_held_block,
                    sync_held_brush,
                    animate_viewmodel,
                )
                    .chain()
                    .after(BlockTargetingSet::PlacementState)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}
