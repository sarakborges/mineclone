mod app;
mod gameplay;
mod hud;
mod player;
mod rendering;
mod screens;
mod targeting;
mod voxel;
mod world;

use app::game_state::GameState;
use bevy::prelude::*;
use gameplay::GameplayPlugin;
use hud::HudPlugin;
use rendering::RenderingPlugin;
use screens::starting_screen::StartingScreenPlugin;
use targeting::block::BlockTargetingPlugin;
use world::WorldPlugin;

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
        .add_plugins((
            StartingScreenPlugin,
            WorldPlugin,
            GameplayPlugin,
            RenderingPlugin,
            BlockTargetingPlugin,
            HudPlugin,
        ))
        .run();
}
