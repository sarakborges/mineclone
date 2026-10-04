mod banner;
mod compass;
mod coordinates;
mod layout;
mod named;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::dimension::DimensionRegistry,
    world::dimension::CurrentDimension,
};
use compass::update_compass_hud;
use coordinates::update_coordinates_hud;
use layout::spawn_world_hud;
use named::{DimensionHudText, update_localized_name_hud};

pub struct WorldHudPlugin;

impl Plugin for WorldHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_world_hud)
            .add_systems(
                Update,
                (
                    update_localized_name_hud::<
                        CurrentDimension,
                        DimensionRegistry,
                        DimensionHudText,
                    >,
                    update_coordinates_hud,
                    update_compass_hud,
                )
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}
