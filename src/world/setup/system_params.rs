use bevy::{
    ecs::system::SystemParam,
    platform::collections::HashSet,
    prelude::*,
};

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, creature::CreatureRegistry,
        dimension::DimensionRegistry, fluid::FluidRegistry, layer::LayerRegistry,
        object::ObjectRegistry, player::PlayerDefinition, structure::StructureRegistry,
        structure_set::StructureSetRegistry,
    },
    rendering::GameplayAssetPreloads,
    voxel::{lighting::PendingLightingUpdates, world::VoxelWorld},
    world::{
        InMemoryWorldSave, NewWorldConfig, WorldLoadMode, WorldSeed,
        chunk_async_work::ChunkAsyncWorkLimiter,
        chunk_generation_tasks::ChunkGenerationTasks,
        chunk_mesh_tasks::ChunkMeshTasks,
        chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
        dimension::CurrentDimension,
        fluid_updates::PendingFluidUpdates,
        game_rules::GameRules,
        render_distance::RenderDistanceSettings,
    },
};

use super::WorldLoadingState;

#[derive(SystemParam)]
pub(in crate::world) struct WorldBootstrapContent<'w> {
    pub(super) asset_server: Res<'w, AssetServer>,
    pub(super) dimensions: Res<'w, DimensionRegistry>,
    pub(super) biomes: Res<'w, BiomeRegistry>,
    pub(super) blocks: Res<'w, BlockRegistry>,
    pub(super) layers: Res<'w, LayerRegistry>,
    pub(super) fluids: Res<'w, FluidRegistry>,
    pub(super) structures: Res<'w, StructureRegistry>,
    pub(super) structure_sets: Res<'w, StructureSetRegistry>,
    pub(super) creatures: Res<'w, CreatureRegistry>,
    pub(super) objects: Res<'w, ObjectRegistry>,
    pub(super) player: Res<'w, PlayerDefinition>,
}

#[derive(SystemParam)]
pub(in crate::world) struct WorldBootstrapConfig<'w> {
    pub(super) current_dimension: Res<'w, CurrentDimension>,
    pub(super) seed: Res<'w, WorldSeed>,
    pub(super) render_distance: Res<'w, RenderDistanceSettings>,
    pub(super) game_rules: ResMut<'w, GameRules>,
}

#[derive(SystemParam)]
pub(in crate::world) struct WorldBootstrapPersistence<'w> {
    pub(super) load_mode: Res<'w, WorldLoadMode>,
    pub(super) new_world_config: Res<'w, NewWorldConfig>,
    pub(super) save: ResMut<'w, InMemoryWorldSave>,
    pub(super) existing_world: Option<Res<'w, VoxelWorld>>,
}

#[derive(SystemParam)]
pub(in crate::world) struct WorldSetupProgress<'w> {
    pub(super) world: ResMut<'w, VoxelWorld>,
    pub(super) loading_state: ResMut<'w, WorldLoadingState>,
}

#[derive(SystemParam)]
pub(in crate::world) struct WorldSetupSimulation<'w, 's> {
    pub(super) fluids: ResMut<'w, PendingFluidUpdates>,
    pub(super) lighting: ResMut<'w, PendingLightingUpdates>,
    pub(super) changed_lighting_chunks: Local<'s, HashSet<IVec3>>,
}

#[derive(SystemParam)]
pub(in crate::world) struct WorldSetupPersistence<'w> {
    pub(super) load_mode: Res<'w, WorldLoadMode>,
    pub(super) new_world_config: Res<'w, NewWorldConfig>,
    pub(super) save: Res<'w, InMemoryWorldSave>,
}

#[derive(SystemParam)]
pub(in crate::world) struct WorldSetupChunkPipeline<'w, 's> {
    pub(super) generation: ChunkGeneration<'w>,
    pub(super) content: ChunkContent<'w>,
    pub(super) renderer: ChunkRenderer<'w, 's>,
    pub(super) generation_tasks: ResMut<'w, ChunkGenerationTasks>,
    pub(super) mesh_tasks: ResMut<'w, ChunkMeshTasks>,
    pub(super) async_work: Res<'w, ChunkAsyncWorkLimiter>,
}

#[derive(SystemParam)]
pub(in crate::world) struct WorldSetupAssets<'w> {
    pub(super) images: ResMut<'w, Assets<Image>>,
    pub(super) asset_server: Res<'w, AssetServer>,
    pub(super) gameplay_preloads: Res<'w, GameplayAssetPreloads>,
}
