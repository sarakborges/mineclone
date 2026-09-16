use bevy::prelude::*;

use crate::{app::game_state::GameState, player::camera::GameplayCamera};
use attachment::attach_fog;
use color::update_fog_color;
use distance::update_fog_distance;

use super::environment::EnvironmentVisualState;

mod attachment;
mod color;
mod distance;

pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (attach_fog.run_if(camera_needs_fog), update_fog_distance)
                .chain()
                .run_if(in_state(GameState::Gameplay)),
        )
        .add_systems(
            PostUpdate,
            update_fog_color
                .run_if(in_state(GameState::Gameplay))
                .run_if(fog_color_inputs_changed),
        );
    }
}

fn camera_needs_fog(cameras: Query<(), (With<GameplayCamera>, Without<DistanceFog>)>) -> bool {
    !cameras.is_empty()
}

fn fog_color_inputs_changed(visuals: Res<EnvironmentVisualState>) -> bool {
    visuals.is_changed()
}
