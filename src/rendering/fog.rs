use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::camera::GameplayCamera,
    world::render_distance::RenderDistanceSettings,
};
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
            attach_fog
                .run_if(in_state(GameState::Gameplay))
                .run_if(camera_needs_fog),
        )
        .add_systems(
            PostUpdate,
            update_fog_color
                .run_if(in_state(GameState::Gameplay))
                .run_if(fog_color_inputs_changed),
        )
        .add_systems(
            PostUpdate,
            update_fog_distance
                .run_if(in_state(GameState::Gameplay))
                .run_if(fog_distance_inputs_changed),
        );
    }
}

fn camera_needs_fog(
    cameras: Query<(), (With<GameplayCamera>, Without<DistanceFog>)>,
) -> bool {
    !cameras.is_empty()
}

fn fog_color_inputs_changed(visuals: Res<EnvironmentVisualState>) -> bool {
    visuals.is_changed()
}

fn fog_distance_inputs_changed(render_distance: Res<RenderDistanceSettings>) -> bool {
    render_distance.is_changed()
}
