pub mod biome;
pub mod biome_field;
pub(crate) mod cave_connectivity;
mod chunk_loading;
pub(crate) mod chunk_rendering;
pub(crate) mod chunk_system_params;
mod chunk_unloading;
pub mod day_night;
mod density_pipeline;
pub mod dimension;
pub(crate) mod feature_graph;
mod generation;
mod generation_pipeline;
pub(crate) mod generation_region;
pub(crate) mod geology;
pub(crate) mod hydrology;
mod macro_climate;
mod material_field;
pub(crate) mod render_distance;
mod save;
mod seed;
mod setup;
mod streaming;
pub(crate) mod terrain;
pub(crate) mod world_feature_fields;

use bevy::prelude::*;

use crate::app::game_state::GameState;
use biome::{CurrentBiome, track_current_biome};
use chunk_rendering::{ChunkRenderPool, clear_chunk_render_pool};
use chunk_unloading::unload_chunk_meshes;
use day_night::DayNightPlugin;
use dimension::CurrentDimension;
use render_distance::RenderDistanceSettings;
pub(crate) use save::{InMemoryWorldSave, WorldLoadMode};
pub(crate) use seed::WorldSeed;
pub(crate) use setup::WorldLoadingState;
use setup::{begin_world_loading, setup_world};
use streaming::{ChunkStreamingState, reset_chunk_streaming, stream_chunks};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentDimension>()
            .init_resource::<CurrentBiome>()
            .init_resource::<WorldSeed>()
            .init_resource::<WorldLoadMode>()
            .init_resource::<InMemoryWorldSave>()
            .init_resource::<RenderDistanceSettings>()
            .init_resource::<ChunkStreamingState>()
            .init_resource::<ChunkRenderPool>()
            .add_plugins(DayNightPlugin)
            .add_systems(OnEnter(GameState::Loading), begin_world_loading)
            .add_systems(OnEnter(GameState::Gameplay), reset_chunk_streaming)
            .add_systems(OnExit(GameState::Gameplay), clear_chunk_render_pool)
            .add_systems(Update, setup_world.run_if(in_state(GameState::Loading)))
            .add_systems(
                Update,
                (unload_chunk_meshes, stream_chunks, track_current_biome)
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}
