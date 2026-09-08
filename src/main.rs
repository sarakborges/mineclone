mod block_targeting;
mod crosshair;
mod game_state;
mod gameplay;
mod gameplay_scene;
mod player;
mod player_camera;
mod player_movement;
mod starting_screen;
mod target_highlight;
mod target_hud;
mod test_chunk;
mod voxel_chunk;
mod voxel_collision;
mod voxel_mesh;
mod voxel_raycast;

use bevy::prelude::*;
use block_targeting::BlockTargetingPlugin;
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
        .add_plugins((StartingScreenPlugin, GameplayPlugin, BlockTargetingPlugin))
        .run();
}
