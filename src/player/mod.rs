pub(crate) mod camera;
pub(crate) mod hotbar;
pub(crate) mod movement;
pub(crate) mod viewmodel;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, dimension::DimensionRegistry},
    world::{
        InMemoryWorldSave, WorldLoadMode, biome_field::BiomeField, dimension::CurrentDimension,
        terrain::surface_height,
    },
};
use camera::GameplayCamera;
use movement::{
    flight::FlightState, gravity::GravityState, swimming::SwimmingState, walking::WalkingState,
};

pub(crate) const PLAYER_HEIGHT: f32 = 1.8;
pub(crate) const PLAYER_EYE_HEIGHT: f32 = 1.62;
pub(crate) const PLAYER_HALF_WIDTH: f32 = 0.3;

const SPAWN_X: i32 = 8;
const SPAWN_Z: i32 = 8;

pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_player);
    }
}

fn spawn_player(
    mut commands: Commands,
    current_dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    load_mode: Res<WorldLoadMode>,
    save: Res<InMemoryWorldSave>,
) {
    let translation = if *load_mode == WorldLoadMode::Load {
        save.player_position().unwrap_or_else(|| {
            default_spawn_position(&current_dimension, &dimensions, &biomes, &biome_field)
        })
    } else {
        default_spawn_position(&current_dimension, &dimensions, &biomes, &biome_field)
    };

    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        Transform::from_translation(translation),
        GameplayCamera::default(),
        WalkingState::default(),
        FlightState::default(),
        GravityState::default(),
        SwimmingState::default(),
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn default_spawn_position(
    current_dimension: &CurrentDimension,
    dimensions: &DimensionRegistry,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> Vec3 {
    let dimension = dimensions
        .get(&current_dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", current_dimension.id));
    let feet_y =
        surface_height(IVec2::new(SPAWN_X, SPAWN_Z), dimension, biomes, biome_field) as f32;

    Vec3::new(SPAWN_X as f32, feet_y + PLAYER_EYE_HEIGHT, SPAWN_Z as f32)
}
