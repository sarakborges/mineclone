mod game_state;
mod gameplay;
mod starting_screen;

use bevy::prelude::*;
use game_state::GameState;
use gameplay::GameplayPlugin;
use starting_screen::StartingScreenPlugin;

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
        .add_plugins((StartingScreenPlugin, GameplayPlugin))
        .run();
}
