mod animation;
mod held_brush;
mod held_chisel;
mod model;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState, resource_systems::reset_resource},
    player::camera::CameraPerspective,
    targeting::block::BlockTargetingSet,
};

pub(crate) use animation::ViewModelAnimation;
use animation::{PlayerViewModel, ViewModelItemSwitch, advance_item_switch, animate_viewmodel};
use held_brush::{setup_held_brush_assets, spawn_held_brush, sync_held_brush};
use held_chisel::{setup_held_chisel_assets, spawn_held_chisel, sync_held_chisel};
use model::{attach_viewmodel_arm_model, spawn_viewmodel, sync_held_block};

pub struct PlayerViewModelPlugin;

impl Plugin for PlayerViewModelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewModelAnimation>()
            .init_resource::<ViewModelItemSwitch>()
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
                Update,
                (
                    spawn_viewmodel,
                    attach_viewmodel_arm_model,
                    spawn_held_brush,
                    spawn_held_chisel,
                    advance_item_switch,
                    sync_held_block,
                    sync_held_brush,
                    sync_held_chisel,
                    animate_viewmodel,
                    sync_viewmodel_visibility,
                )
                    .chain()
                    .after(BlockTargetingSet::PlacementState)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}


fn sync_viewmodel_visibility(
    perspective: Res<CameraPerspective>,
    pause: Res<State<PauseState>>,
    mut viewmodels: Query<&mut Visibility, With<PlayerViewModel>>,
) {
    let next_visibility =
        if !perspective.is_third_person() && *pause.get() == PauseState::Running {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

    for mut visibility in &mut viewmodels {
        if *visibility != next_visibility {
            *visibility = next_visibility;
        }
    }
}
