pub(crate) mod biome;
pub(crate) mod biome_field;
pub(crate) mod cave_connectivity;
mod chunk_generation_tasks;
mod chunk_mesh_tasks;
pub(crate) mod chunk_remesh;
mod chunk_remesh_tasks;
pub(crate) mod chunk_rendering;
pub(crate) mod chunk_system_params;
mod chunk_task_queue;
mod chunk_unloading;
mod chunk_visibility;
pub(crate) mod current_context;
pub(crate) mod day_night;
mod density_sampling;
mod deterministic;
pub(crate) mod dimension;
pub(crate) mod feature_graph;
pub(crate) mod fluid_updates;
pub(crate) mod game_rules;
mod generation;
pub(crate) mod generation_region;
pub(crate) mod hydrology;
mod lighting_updates;
mod macro_climate;
mod material_field;
pub(crate) mod math;
pub(crate) mod new_world;
mod noise;
mod render_diagnostics;
pub(crate) mod render_distance;
mod save;
mod seed;
mod setup;
mod streaming;
pub(crate) mod terrain;
pub(crate) mod tick;
mod work_budget;
pub(crate) mod world_feature_fields;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    voxel::lighting::PendingLightingUpdates,
};
use biome::{CurrentBiome, track_current_biome};
use chunk_generation_tasks::ChunkGenerationTasks;
use chunk_mesh_tasks::ChunkMeshTasks;
use chunk_remesh::{
    ChunkRemeshQueue, process_chunk_remesh_queue, process_immediate_geometry_remesh,
};
use chunk_remesh_tasks::ChunkRemeshTasks;
use chunk_rendering::{ChunkRenderPool, clear_chunk_render_pool};
use chunk_unloading::{ChunkUnloadState, unload_chunk_meshes};
use chunk_visibility::{sync_chunk_visibility, sync_new_chunk_visibility};
use day_night::DayNightPlugin;
use dimension::CurrentDimension;
use fluid_updates::{PendingFluidUpdates, process_fluid_updates};
use game_rules::GameRules;
use lighting_updates::{pending_lighting_work, process_dynamic_lighting};
pub(crate) use new_world::NewWorldConfig;
use render_diagnostics::{log_render_asset_pressure, render_diagnostics_due};
use render_distance::RenderDistanceSettings;
pub(crate) use save::{InMemoryWorldSave, WorldLoadMode};
pub(crate) use seed::WorldSeed;
pub(crate) use setup::WorldLoadingState;
use setup::{begin_world_loading, setup_world};
use streaming::{ChunkStreamingState, stream_chunks};
use tick::{WorldTickClock, WorldTickSet, advance_world_ticks, world_ticks_advanced};

pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentDimension>()
            .init_resource::<CurrentBiome>()
            .init_resource::<WorldSeed>()
            .init_resource::<WorldLoadMode>()
            .init_resource::<InMemoryWorldSave>()
            .init_resource::<NewWorldConfig>()
            .init_resource::<GameRules>()
            .init_resource::<WorldTickClock>()
            .init_resource::<RenderDistanceSettings>()
            .init_resource::<ChunkStreamingState>()
            .init_resource::<ChunkGenerationTasks>()
            .init_resource::<ChunkMeshTasks>()
            .init_resource::<ChunkRemeshTasks>()
            .init_resource::<ChunkUnloadState>()
            .init_resource::<ChunkRenderPool>()
            .init_resource::<ChunkRemeshQueue>()
            .init_resource::<PendingLightingUpdates>()
            .init_resource::<PendingFluidUpdates>()
            .add_plugins(DayNightPlugin)
            .add_systems(
                OnEnter(GameState::Loading),
                (
                    reset_resource::<ChunkGenerationTasks>,
                    reset_resource::<ChunkMeshTasks>,
                    reset_resource::<ChunkRemeshTasks>,
                    begin_world_loading,
                )
                    .chain(),
            )
            .add_systems(
                OnEnter(GameState::Gameplay),
                (
                    reset_resource::<ChunkStreamingState>,
                    reset_resource::<ChunkGenerationTasks>,
                    reset_resource::<ChunkMeshTasks>,
                    reset_resource::<ChunkRemeshTasks>,
                    reset_resource::<ChunkUnloadState>,
                    reset_resource::<WorldTickClock>,
                ),
            )
            .add_systems(
                OnExit(GameState::Gameplay),
                (
                    clear_chunk_render_pool,
                    reset_resource::<ChunkGenerationTasks>,
                    reset_resource::<ChunkMeshTasks>,
                    reset_resource::<ChunkRemeshTasks>,
                    reset_resource::<ChunkUnloadState>,
                    reset_resource::<ChunkRemeshQueue>,
                    reset_resource::<PendingLightingUpdates>,
                    reset_resource::<PendingFluidUpdates>,
                ),
            )
            .add_systems(Update, setup_world.run_if(in_state(GameState::Loading)))
            .add_systems(
                PreUpdate,
                advance_world_ticks
                    .in_set(WorldTickSet)
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                (stream_chunks, unload_chunk_meshes)
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                track_current_biome.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                PostUpdate,
                (
                    process_immediate_geometry_remesh,
                    process_fluid_updates.run_if(world_ticks_advanced),
                    process_dynamic_lighting.run_if(pending_lighting_work),
                    process_chunk_remesh_queue,
                    sync_chunk_visibility,
                    sync_new_chunk_visibility,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Last,
                log_render_asset_pressure.run_if(render_diagnostics_due),
            );
    }
}
