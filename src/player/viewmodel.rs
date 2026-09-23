mod animation;
mod held_brush;
mod held_artisans_kit;
mod model;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState, resource_systems::reset_resource},
    player::camera::CameraPerspective,
    targeting::block::BlockTargetingSet,
};

pub(crate) use animation::ViewModelAnimation;
use animation::{
    PlayerViewModel, ViewModelItemSwitch, advance_item_switch, advance_viewmodel_animation,
    animate_viewmodel,
};
use held_brush::{setup_held_brush_assets, spawn_held_brush, sync_held_brush};
use held_artisans_kit::{setup_held_artisans_kit_assets, spawn_held_artisans_kit, sync_held_artisans_kit};
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
                    setup_held_artisans_kit_assets,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    spawn_viewmodel,
                    attach_viewmodel_arm_model,
                    spawn_held_brush,
                    spawn_held_artisans_kit,
                    advance_item_switch,
                    sync_held_block,
                    sync_held_brush,
                    sync_held_artisans_kit,
                    animate_viewmodel,
                    sync_viewmodel_visibility,
                )
                    .chain()
                    .after(BlockTargetingSet::Interaction)
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                PostUpdate,
                advance_viewmodel_animation.run_if(in_state(GameState::Gameplay)),
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
