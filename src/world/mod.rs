pub mod biome;
pub mod biome_field;
mod chunk_rendering;
pub mod day_night;
pub mod dimension;
pub(crate) mod render_distance;
mod setup;
mod streaming;
mod test_world;

use bevy::prelude::*;

use crate::app::game_state::GameState;
use biome::{track_current_biome, CurrentBiome};
use day_night::DayNightPlugin;
use dimension::CurrentDimension;
use render_distance::RenderDistanceSettings;
use setup::{begin_world_loading, setup_world};
use streaming::{reset_chunk_streaming, stream_chunks, ChunkStreamingState};
pub(crate) use setup::WorldLoadingState;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentDimension>()
            .init_resource::<CurrentBiome>()
            .init_resource::<RenderDistanceSettings>()
            .init_resource::<ChunkStreamingState>()
            .add_plugins(DayNightPlugin)
            .add_systems(OnEnter(GameState::Loading), begin_world_loading)
            .add_systems(OnEnter(GameState::Gameplay), reset_chunk_streaming)
            .add_systems(
                Update,
                setup_world.run_if(in_state(GameState::Loading)),
            )
            .add_systems(
                Update,
                (stream_chunks, track_current_biome).run_if(in_state(GameState::Gameplay)),
            );
    }
}
