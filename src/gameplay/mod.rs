use bevy::prelude::*;

use crate::player::{camera::PlayerCameraPlugin, movement::PlayerMovementPlugin, PlayerPlugin};

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((PlayerPlugin, PlayerCameraPlugin, PlayerMovementPlugin));
    }
}
