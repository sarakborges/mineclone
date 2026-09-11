use bevy::prelude::*;

use crate::app::game_state::GameState;
use attachment::attach_fog;
use color::update_fog_color;
use distance::update_fog_distance;
use material::sync_terrain_fog;

mod attachment;
mod color;
mod distance;
mod material;

pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                attach_fog,
                update_fog_color,
                update_fog_distance,
                sync_terrain_fog,
            )
                .chain()
                .run_if(in_state(GameState::Gameplay)),
        );
    }
}
