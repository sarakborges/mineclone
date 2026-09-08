mod starting_screen;

use bevy::prelude::*;
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
        .add_plugins(StartingScreenPlugin)
        .run();
}
