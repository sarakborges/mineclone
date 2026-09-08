use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, block::BlockRegistry, dimension::DimensionRegistry},
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
    voxel::{chunk::CHUNK_SIZE, mesh::build_chunk_mesh, world::VoxelWorld},
};

use super::{
    biome_field::BiomeField,
    dimension::CurrentDimension,
    render_distance::{chunk_coords_in_radius, RenderDistanceSettings},
    test_world::build_test_chunk,
};

const GRASS_BLOCK_ID: &str = "mineclone:grass";

#[derive(Resource)]
pub struct WorldLoadingState {
    coords: Vec<IVec2>,
    generated: usize,
    screen_rendered: bool,
    transition_requested: bool,
    material: Handle<StandardMaterial>,
}

impl WorldLoadingState {
    pub fn generated(&self) -> usize {
        self.generated
    }

    pub fn total(&self) -> usize {
        self.coords.len()
    }
}

pub fn begin_world_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    current_dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    biomes: Res<BiomeRegistry>,
    blocks: Res<BlockRegistry>,
    render_distance: Res<RenderDistanceSettings>,
) {
    let dimension = dimensions
        .get(&current_dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", current_dimension.id));
    let grass = blocks
        .get(GRASS_BLOCK_ID)
        .unwrap_or_else(|| panic!("missing block definition: {GRASS_BLOCK_ID}"));
    let biome_field = BiomeField::from_dimension(dimension, &biomes);
    let (roughness, metallic) = average_terrain_material(dimension, &biomes);
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(asset_server.load(&grass.texture)),
        perceptual_roughness: roughness,
        metallic,
        ..default()
    });
    let coords = chunk_coords_in_radius(IVec2::ZERO, render_distance.chunks());

    commands.insert_resource(VoxelWorld::default());
    commands.insert_resource(biome_field);
    commands.insert_resource(WorldLoadingState {
        coords,
        generated: 0,
        screen_rendered: false,
        transition_requested: false,
        material,
    });
}

pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    blocks: Res<BlockRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    mut world: ResMut<VoxelWorld>,
    mut loading_state: ResMut<WorldLoadingState>,
    mut transition: ResMut<ScreenTransition>,
) {
    if !loading_state.screen_rendered {
        loading_state.screen_rendered = true;
        return;
    }

    if loading_state.generated >= loading_state.coords.len() {
        if !loading_state.transition_requested {
            loading_state.transition_requested = true;
            transition.request(ScreenTransitionTarget::game(GameState::Gameplay));
        }
        return;
    }

    let coord = loading_state.coords[loading_state.generated];
    let chunk = build_test_chunk(coord, &blocks);
    world.insert_chunk(coord, chunk);

    let chunk = world
        .chunk(coord)
        .unwrap_or_else(|| panic!("generated chunk should exist at {coord}"));
    let mesh = meshes.add(build_chunk_mesh(&world, coord, chunk, |voxel| {
        let position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
        let grass = biome_field.grass_color(position, &biomes);
        [grass.r, grass.g, grass.b]
    }));
    let chunk_size = CHUNK_SIZE as f32;

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(loading_state.material.clone()),
        Transform::from_xyz(
            coord.x as f32 * chunk_size,
            0.0,
            coord.y as f32 * chunk_size,
        ),
        DespawnOnExit(GameState::Gameplay),
    ));

    loading_state.generated += 1;
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
