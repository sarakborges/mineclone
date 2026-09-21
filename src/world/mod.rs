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
mod chunk_storage;
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
mod storage_durability;
mod streaming;
pub(crate) mod terrain;
pub(crate) mod tick;
mod work_budget;
pub(crate) mod warp;
pub(crate) mod world_feature_fields;
pub(crate) mod world_names;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    player::hotbar::PlayerHotbar,
    rendering::terrain_material::TerrainLightingBuffer,
    voxel::{lighting::PendingLightingUpdates, world::VoxelWorld},
};
use biome::{CurrentBiome, track_current_biome};
use biome_field::BiomeField;
use chunk_generation_tasks::ChunkGenerationTasks;
use chunk_mesh_tasks::ChunkMeshTasks;
use chunk_remesh::{ChunkRemeshQueue, process_chunk_remesh_queue};
use chunk_remesh_tasks::ChunkRemeshTasks;
use chunk_rendering::{
    ChunkRenderPool, FluidMaterials, TerrainMaterials, clear_chunk_render_pool,
};
use chunk_unloading::{ChunkUnloadState, retire_distant_chunk_meshes, unload_chunk_meshes};
use chunk_visibility::{sync_chunk_visibility, sync_new_chunk_visibility};
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
use save_catalog::WorldDirectoryLock;
use save_session::{
    WorldSession, exit_on_window_close_without_gameplay, restore_loaded_clock,
    save_on_gameplay_window_close,
};
pub(crate) use seed::WorldSeed;
pub(crate) use setup::WorldLoadingState;
use setup::{begin_world_loading, setup_world};
use streaming::{ChunkStreamingState, stream_chunks};
use warp::{PendingWarp, resolve_pending_warp};
use tick::{WorldTickClock, WorldTickSet, advance_world_ticks};
use work_budget::{WorldFrameWorkBudget, begin_world_frame_work_budget};
use world_feature_fields::WorldFeatureFields;

pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentDimension>()
            .init_resource::<DimensionEntityCounts>()
            .init_resource::<CurrentBiome>()
            .init_resource::<WorldSeed>()
            .init_resource::<WorldLoadMode>()
            .init_resource::<InMemoryWorldSave>()
            .init_resource::<WorldSession>()
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
            .init_resource::<PendingWarp>()
            .init_resource::<WorldFrameWorkBudget>()
            .add_plugins(DayNightPlugin)
            .add_systems(OnEnter(GameState::StartingScreen), release_world_session)
            .add_systems(
                OnEnter(GameState::Loading),
                (
                    reset_resource::<ChunkGenerationTasks>,
                    reset_resource::<ChunkMeshTasks>,
                    reset_resource::<ChunkRemeshTasks>,
                    reset_resource::<ChunkRemeshQueue>,
                    reset_resource::<PendingLightingUpdates>,
                    reset_resource::<PendingFluidUpdates>,
                    reset_resource::<PendingWarp>,
                    prepare_world_session,
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
                    reset_resource::<PendingWarp>,
                    restore_loaded_clock,
                    reseed_loaded_fluid_frontiers,
                )
                    .chain(),
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
                    reset_resource::<PendingWarp>,
                ),
            )
            .add_systems(Update, setup_world.run_if(in_state(GameState::Loading)))
            .add_systems(
                PreUpdate,
                (
                    begin_world_frame_work_budget,
                    advance_world_ticks.in_set(WorldTickSet),
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                (
                    stream_chunks,
                    retire_distant_chunk_meshes,
                    resolve_pending_warp,
                    unload_chunk_meshes,
                )
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
                    process_fluid_updates,
                    process_dynamic_lighting.run_if(pending_lighting_work),
                    process_chunk_remesh_queue,
                    sync_chunk_visibility,
                    sync_new_chunk_visibility,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(Last, log_render_asset_pressure.run_if(render_diagnostics_due))
            .add_systems(Last, exit_on_window_close_without_gameplay)
            .add_systems(
                Last,
                save_on_gameplay_window_close.run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn prepare_world_session(
    mut session: ResMut<WorldSession>,
    mode: Res<WorldLoadMode>,
    config: Res<NewWorldConfig>,
) {
    if *mode == WorldLoadMode::New {
        *session = WorldSession::new(config.name().to_owned());
    }
}

/// Called only upon returning to the starting screen, after the Leave World
/// action has successfully published the snapshot. Never drop this state in
/// the error path: the player must be able to retry the save.
fn release_world_session(mut commands: Commands) {
    commands.remove_resource::<VoxelWorld>();
    commands.remove_resource::<BiomeField>();
    commands.remove_resource::<WorldFeatureFields>();
    commands.remove_resource::<TerrainLightingBuffer>();
    commands.remove_resource::<TerrainMaterials>();
    commands.remove_resource::<FluidMaterials>();
    commands.remove_resource::<WorldLoadingState>();
    commands.remove_resource::<WorldDirectoryLock>();
    commands.insert_resource(InMemoryWorldSave::default());
    commands.insert_resource(WorldSession::default());
    commands.insert_resource(PlayerHotbar::default());
}
