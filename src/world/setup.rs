use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, dimension::DimensionRegistry},
    voxel::{chunk::CHUNK_SIZE, mesh::build_chunk_mesh},
};

use super::{
    biome_field::BiomeField,
    dimension::CurrentDimension,
    render_distance::RENDER_DISTANCE_RADIUS,
    test_world::build_test_world,
};

pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    current_dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let dimension = dimensions
        .get(&current_dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", current_dimension.id));
    let biome_field = BiomeField::from_dimension(dimension, &biomes);
    let center = IVec2::ZERO;
    let world = build_test_world(center, RENDER_DISTANCE_RADIUS);
    let (roughness, metallic) = average_terrain_material(dimension, &biomes);
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: roughness,
        metallic,
        ..default()
    });
    let chunk_size = CHUNK_SIZE as f32;

    for (coord, chunk) in world.chunks() {
        let mesh = meshes.add(build_chunk_mesh(&world, *coord, chunk, |voxel| {
            let position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
            let grass = biome_field.grass_color(position, &biomes);
            [grass.r, grass.g, grass.b]
        }));

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

    commands.insert_resource(biome_field);
    commands.insert_resource(world);
}

fn average_terrain_material(
    dimension: &crate::content::dimension::DimensionDefinition,
    biomes: &BiomeRegistry,
) -> (f32, f32) {
    let mut roughness = 0.0;
    let mut metallic = 0.0;
    let mut count = 0.0;

    for biome_id in &dimension.biomes {
        let biome = biomes
            .get(biome_id)
            .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));
        roughness += biome.visuals.terrain_roughness;
        metallic += biome.visuals.terrain_metallic;
        count += 1.0;
    }

    (roughness / count, metallic / count)
}
