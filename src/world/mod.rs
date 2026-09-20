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
#[cfg(test)]
mod chunk_storage;
mod chunk_unloading;
mod chunk_visibility;
mod clock_persistence;
pub(crate) mod current_context;
pub(crate) mod day_night;
mod density_sampling;
mod deterministic;
pub(crate) mod dimension;
pub(crate) mod feature_graph;
pub(crate) mod fluid_updates;
pub(crate) mod game_rules;
pub(crate) mod generation;
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
pub(crate) mod save_catalog;
pub(crate) mod save_session;
mod seed;
mod setup;
mod streaming;
pub(crate) mod terrain;
pub(crate) mod tick;
mod work_budget;
pub(crate) mod world_feature_fields;
pub(crate) mod world_names;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    player::hotbar::PlayerHotbar,
    voxel::{lighting::PendingLightingUpdates, world::VoxelWorld},
};
use biome::{CurrentBiome, track_current_biome};
use biome_field::BiomeField;
use chunk_generation_tasks::ChunkGenerationTasks;
use chunk_mesh_tasks::ChunkMeshTasks;
use chunk_remesh::{
    ChunkRemeshQueue, process_chunk_remesh_queue, process_immediate_geometry_remesh,
};
use chunk_remesh_tasks::ChunkRemeshTasks;
use chunk_rendering::{
    ChunkRenderPool, FluidMaterials, TerrainMaterials, clear_chunk_render_pool,
};
use chunk_unloading::{ChunkUnloadState, unload_chunk_meshes};
use chunk_visibility::{sync_chunk_visibility, sync_new_chunk_visibility};
use clock_persistence::{
    ClockPersistence, persist_clock_periodically, reset_clock_persistence, restore_persisted_clock,
};
use day_night::DayNightPlugin;
use dimension::{CurrentDimension, DimensionEntityCounts};
use fluid_updates::{PendingFluidUpdates, process_fluid_updates, reseed_loaded_fluid_frontiers};
use game_rules::GameRules;
use lighting_updates::{pending_lighting_work, process_dynamic_lighting};
pub(crate) use new_world::{
    DEFAULT_BIOME_SIZE_MULTIPLIER,
    MAX_BIOME_SIZE_MULTIPLIER, MIN_BIOME_SIZE_MULTIPLIER, NewWorldConfig,
    is_valid_biome_size_multiplier, snap_biome_size_multiplier,
};
use render_diagnostics::{log_render_asset_pressure, render_diagnostics_due};
use render_distance::RenderDistanceSettings;
pub(crate) use save::{InMemoryWorldSave, WorldLoadMode};
use save::{
    LoadedWorld, SaveState, apply_loaded_world, autosave_world, begin_world_load,
    commit_world_on_exit_request, commit_world_on_leave, finalize_world_load,
    initialize_new_world_save, release_world_session, reset_save_state, save_on_pause_entry,
};
use save_session::{SaveSession, reset_save_session};
use seed::WorldSeed;
use setup::{setup_world, teardown_world};
use streaming::{ChunkStreamingConfig, update_chunk_streaming};
use tick::WorldTickClock;
use work_budget::FrameWorkBudget;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DayNightPlugin)
            .init_resource::<WorldSeed>()
            .init_resource::<VoxelWorld>()
            .init_resource::<BiomeField>()
            .init_resource::<CurrentBiome>()
            .init_resource::<CurrentDimension>()
            .init_resource::<DimensionEntityCounts>()
            .init_resource::<GameRules>()
            .init_resource::<RenderDistanceSettings>()
            .init_resource::<ChunkStreamingConfig>()
            .init_resource::<ChunkGenerationTasks>()
            .init_resource::<ChunkTaskQueue>()
            .init_resource::<ChunkMeshTasks>()
            .init_resource::<ChunkRemeshTasks>()
            .init_resource::<ChunkRemeshQueue>()
            .init_resource::<ChunkUnloadState>()
            .init_resource::<ChunkRenderPool>()
            .init_resource::<TerrainMaterials>()
            .init_resource::<FluidMaterials>()
            .init_resource::<PendingFluidUpdates>()
            .init_resource::<PendingLightingUpdates>()
            .init_resource::<LightingRemeshState>()
            .init_resource::<ClockPersistence>()
            .init_resource::<WorldTickClock>()
            .init_resource::<SaveState>()
            .init_resource::<SaveSession>()
            .init_resource::<LoadedWorld>()
            .add_systems(OnEnter(GameState::LoadingWorld), begin_world_load)
            .add_systems(
                Update,
                finalize_world_load.run_if(in_state(GameState::LoadingWorld)),
            )
            .add_systems(
                OnEnter(GameState::Gameplay),
                (
                    initialize_new_world_save,
                    apply_loaded_world,
                    setup_world,
                    restore_persisted_clock,
                    reseed_loaded_fluid_frontiers,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    update_chunk_streaming,
                    unload_chunk_meshes,
                    process_dynamic_lighting,
                    process_immediate_geometry_remesh,
                    process_chunk_remesh_queue,
                    sync_new_chunk_visibility,
                    sync_chunk_visibility,
                    track_current_biome,
                    persist_clock_periodically,
                    autosave_world,
                    log_render_asset_pressure.run_if(render_diagnostics_due),
                )
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(OnEnter(GameState::Paused), save_on_pause_entry)
            .add_systems(OnExit(GameState::Gameplay), commit_world_on_leave)
            .add_systems(
                OnEnter(GameState::MainMenu),
                (
                    teardown_world,
                    clear_chunk_render_pool,
                    reset_resource::<VoxelWorld>,
                    reset_resource::<BiomeField>,
                    reset_resource::<CurrentBiome>,
                    reset_resource::<CurrentDimension>,
                    reset_resource::<DimensionEntityCounts>,
                    reset_resource::<GameRules>,
                    reset_resource::<ChunkGenerationTasks>,
                    reset_resource::<ChunkTaskQueue>,
                    reset_resource::<ChunkMeshTasks>,
                    reset_resource::<ChunkRemeshTasks>,
                    reset_resource::<ChunkRemeshQueue>,
                    reset_resource::<ChunkUnloadState>,
                    reset_resource::<PendingFluidUpdates>,
                    reset_resource::<PendingLightingUpdates>,
                    reset_resource::<LightingRemeshState>,
                    reset_clock_persistence,
                    reset_resource::<WorldTickClock>,
                    reset_save_state,
                    reset_save_session,
                    release_world_session,
                ),
            )
            .add_systems(
                Last,
                commit_world_on_exit_request.run_if(in_state(GameState::Gameplay)),
            );
    }
}
