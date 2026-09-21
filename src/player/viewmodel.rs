mod animation;
mod held_brush;
mod held_chisel;
mod model;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState, resource_systems::reset_resource},
    targeting::block::BlockTargetingSet,
    ui::visibility::set_visibility,
};

pub(crate) use animation::ViewModelAnimation;
use animation::{PlayerViewModel, ViewModelItemSwitch, advance_item_switch, animate_viewmodel};
use held_brush::{setup_held_brush_assets, spawn_held_brush, sync_held_brush};
use held_chisel::{setup_held_chisel_assets, spawn_held_chisel, sync_held_chisel};
use model::{setup_viewmodel_arm_assets, spawn_viewmodel, sync_held_block};

pub struct PlayerViewModelPlugin;

impl Plugin for PlayerViewModelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewModelAnimation>()
            .init_resource::<ViewModelItemSwitch>()
            .add_systems(Startup, setup_viewmodel_arm_assets)
            .add_systems(
                OnEnter(GameState::Gameplay),
                (
                    reset_resource::<ViewModelAnimation>,
                    reset_resource::<ViewModelItemSwitch>,
                    setup_held_brush_assets,
                    setup_held_chisel_assets,
                )
                    .chain(),
            )
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
                    spawn_held_chisel,
                    advance_item_switch,
                    sync_held_block,
                    sync_held_brush,
                    sync_held_chisel,
                    animate_viewmodel,
                )
                    .chain()
                    .after(BlockTargetingSet::PlacementState)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}
