use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, dimension::DimensionRegistry,
        fluid::FluidRegistry,
    },
    voxel::world::VoxelWorld,
};

use super::WorldLoadingState;
use crate::world::{
    InMemoryWorldSave, NewWorldConfig, WorldLoadMode, WorldSeed,
    dimension::CurrentDimension,
    game_rules::GameRules,
    render_distance::RenderDistanceSettings,
};

#[derive(SystemParam)]
pub(in crate::world) struct WorldBootstrapContent<'w> {
    pub(super) asset_server: Res<'w, AssetServer>,
    pub(super) dimensions: Res<'w, DimensionRegistry>,
    pub(super) biomes: Res<'w, BiomeRegistry>,
    pub(super) blocks: Res<'w, BlockRegistry>,
    pub(super) fluids: Res<'w, FluidRegistry>,
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
pub(in crate::world) struct WorldSetupPersistence<'w> {
    pub(super) load_mode: Res<'w, WorldLoadMode>,
    pub(super) new_world_config: Res<'w, NewWorldConfig>,
    pub(super) save: Res<'w, InMemoryWorldSave>,
}
