use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    voxel::{mesh::build_chunk_mesh, test_chunk::collision_test_chunk},
};

pub struct GameplayScenePlugin;

impl Plugin for GameplayScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), setup_scene);
    }
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let chunk = collision_test_chunk();
    let chunk_mesh = meshes.add(build_chunk_mesh(&chunk));

    commands.spawn((
        Mesh3d(chunk_mesh),
        MeshMaterial3d(materials.add(Color::srgb(0.22, 0.42, 0.2))),
        chunk,
        DespawnOnExit(GameState::Gameplay),
    ));

    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(8.0, 10.0, 8.0),
        DespawnOnExit(GameState::Gameplay),
    ));
}
