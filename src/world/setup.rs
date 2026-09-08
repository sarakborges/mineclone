use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, block::BlockRegistry, dimension::DimensionRegistry},
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
    voxel::world::VoxelWorld,
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::{spawn_chunk_mesh, ChunkRenderPool, TerrainMaterials},
    dimension::CurrentDimension,
    render_distance::chunk_coords_in_cylinder,
    terrain::{build_chunk, chunk_y_bounds},
};

const GRASS_BLOCK_ID: &str = "mineclone:grass";
const INITIAL_HORIZONTAL_RADIUS_CHUNKS: i32 = 4;
const INITIAL_CHUNKS_PER_FRAME: usize = 4;

#[derive(Resource)]
pub struct WorldLoadingState {
    coords: Vec<IVec3>,
    generated: usize,
    screen_rendered: bool,
    transition_requested: bool,
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
) {
    let dimension = dimensions
        .get(&current_dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", current_dimension.id));
    let grass = blocks
        .get(GRASS_BLOCK_ID)
        .unwrap_or_else(|| panic!("missing block definition: {GRASS_BLOCK_ID}"));
    let biome_field = BiomeField::from_dimension(dimension, &biomes);
    let (roughness, metallic) = average_terrain_material(dimension, &biomes);
    let mut create_material = |texture: &str| {
        materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(asset_server.load(texture.to_owned())),
            perceptual_roughness: roughness,
            metallic,
            ..default()
        })
    };
    let terrain_materials = TerrainMaterials {
        top: create_material(&grass.textures.top),
        bottom: create_material(&grass.textures.bottom),
        left: create_material(&grass.textures.left),
        right: create_material(&grass.textures.right),
        front: create_material(&grass.textures.front),
        back: create_material(&grass.textures.back),
    };
    let (min_chunk_y, max_chunk_y) = chunk_y_bounds(dimension, &biomes);
    let coords = chunk_coords_in_cylinder(
        IVec3::new(0, min_chunk_y, 0),
        INITIAL_HORIZONTAL_RADIUS_CHUNKS,
        min_chunk_y,
        max_chunk_y,
    );

    commands.insert_resource(VoxelWorld::default());
    commands.insert_resource(biome_field);
    commands.insert_resource(terrain_materials);
    commands.insert_resource(WorldLoadingState {
        coords,
        generated: 0,
        screen_rendered: false,
        transition_requested: false,
    });
}

pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    current_dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    blocks: Res<BlockRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    materials: Res<TerrainMaterials>,
    mut world: ResMut<VoxelWorld>,
    mut render_pool: ResMut<ChunkRenderPool>,
    mut loading_state: ResMut<WorldLoadingState>,
    mut transition: ResMut<ScreenTransition>,
) {
    if transition.is_active() {
        return;
    }

    if !loading_state.screen_rendered {
        loading_state.screen_rendered = true;
        return;
    }

    let dimension = dimensions
        .get(&current_dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", current_dimension.id));

    for _ in 0..INITIAL_CHUNKS_PER_FRAME {
        if loading_state.generated >= loading_state.coords.len() {
            break;
        }

        let coord = loading_state.coords[loading_state.generated];
        let chunk = build_chunk(coord, &blocks, dimension, &biomes, &biome_field);
        world.insert_chunk(coord, chunk);

        let chunk = world
            .chunk(coord)
            .unwrap_or_else(|| panic!("generated chunk should exist at {coord:?}"));
        spawn_chunk_mesh(
            &mut commands,
            &mut meshes,
            &mut render_pool,
            &world,
            coord,
            chunk,
            &biomes,
            &biome_field,
            &materials,
        );

        loading_state.generated += 1;
    }

    if loading_state.generated >= loading_state.coords.len()
        && !loading_state.transition_requested
    {
        loading_state.transition_requested = true;
        transition.request(ScreenTransitionTarget::game(GameState::Gameplay));
    }
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
