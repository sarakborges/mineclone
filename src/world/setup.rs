use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    voxel::{chunk::CHUNK_SIZE, mesh::build_chunk_mesh},
};

use super::{
    render_distance::RENDER_DISTANCE_RADIUS,
    test_world::build_test_world,
};

pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let center = IVec2::ZERO;
    let world = build_test_world(center, RENDER_DISTANCE_RADIUS);
    let material = materials.add(Color::srgb(0.22, 0.42, 0.2));
    let chunk_size = CHUNK_SIZE as f32;

    for (coord, chunk) in world.chunks() {
        let mesh = meshes.add(build_chunk_mesh(&world, *coord, chunk));

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(
                coord.x as f32 * chunk_size,
                0.0,
                coord.y as f32 * chunk_size,
            ),
            DespawnOnExit(GameState::Gameplay),
        ));
    }

    commands.insert_resource(world);
}
