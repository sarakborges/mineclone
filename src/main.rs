mod app;
mod gameplay;
mod player;
mod screens;
mod targeting;
mod voxel;

use app::game_state::GameState;
use bevy::prelude::*;
use gameplay::GameplayPlugin;
use screens::starting_screen::StartingScreenPlugin;
use targeting::block::BlockTargetingPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Mineclone".into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.025, 0.04)))
        .add_plugins((StartingScreenPlugin, GameplayPlugin, BlockTargetingPlugin))
        .run();
}
