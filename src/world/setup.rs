use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, dimension::DimensionRegistry},
    voxel::{chunk::CHUNK_SIZE, mesh::build_chunk_mesh},
};

use super::{
    biome::CurrentBiome,
    dimension::CurrentDimension,
    render_distance::RENDER_DISTANCE_RADIUS,
    test_world::build_test_world,
};

pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    current_dimension: Res<CurrentDimension>,
    current_biome: Res<CurrentBiome>,
    dimensions: Res<DimensionRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let dimension = dimensions
        .get(&current_dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", current_dimension.id));

    if !dimension.biomes.iter().any(|id| id == &current_biome.id) {
        panic!(
            "biome {} is not eligible for dimension {}",
            current_biome.id, current_dimension.id
        );
    }

    let biome = biomes
        .get(&current_biome.id)
        .unwrap_or_else(|| panic!("missing biome definition: {}", current_biome.id));

    let center = IVec2::ZERO;
    let world = build_test_world(center, RENDER_DISTANCE_RADIUS);
    let material = materials.add(StandardMaterial {
        base_color: biome.visuals.terrain_color.to_color(),
        perceptual_roughness: biome.visuals.terrain_roughness,
        metallic: biome.visuals.terrain_metallic,
        ..default()
    });
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
