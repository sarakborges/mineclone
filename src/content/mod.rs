pub mod biome;
pub mod color;
pub mod day_night_cycle;
pub mod dimension;
mod loader;

use bevy::prelude::*;
use loader::load_content;

pub struct ContentPlugin;

impl Plugin for ContentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_content);
    }
}
