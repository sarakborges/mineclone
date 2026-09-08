pub mod camera;
pub mod movement;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, dimension::DimensionRegistry},
    world::{
        biome_field::BiomeField,
        dimension::CurrentDimension,
        terrain::surface_height,
    },
};
use camera::GameplayCamera;
use movement::{
    flight::FlightState,
    gravity::GravityState,
    walking::WalkingState,
};

pub const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_EYE_HEIGHT: f32 = 1.62;
pub const PLAYER_HALF_WIDTH: f32 = 0.3;

const SPAWN_X: i32 = 8;
const SPAWN_Z: i32 = 8;

pub struct PlayerPlugin;

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
) {
    let dimension = dimensions
        .get(&current_dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", current_dimension.id));
    let feet_y = surface_height(
        IVec2::new(SPAWN_X, SPAWN_Z),
        dimension,
        &biomes,
        &biome_field,
    ) as f32;

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(
            SPAWN_X as f32,
            feet_y + PLAYER_EYE_HEIGHT,
            SPAWN_Z as f32,
        ),
        GameplayCamera::default(),
        WalkingState::default(),
        FlightState::default(),
        GravityState::default(),
        DespawnOnExit(GameState::Gameplay),
    ));
}
