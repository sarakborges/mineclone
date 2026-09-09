use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{
        biome::{BiomeKind, BiomeRegistry},
        block::BlockRegistry,
        dimension::{DimensionDefinition, DimensionRegistry},
        fluid::FluidRegistry,
        read_content,
    },
    rendering::terrain_material::{TerrainMaterial, TerrainMaterialExtension},
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
    voxel::{
        chunk::CHUNK_SIZE,
        coordinates::split_dimension_position,
        world::VoxelWorld,
    },
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::{
        refresh_adjacent_chunk_meshes, spawn_chunk_mesh, ChunkRenderPool, FluidMaterials,
        TerrainMaterials,
    },
    dimension::CurrentDimension,
    generation::generate_chunk,
    render_distance::chunk_coords_in_volume,
    terrain::surface_height,
    InMemoryWorldSave,
    WorldLoadMode,
    WorldSeed,
};

const GRASS_BLOCK_ID: &str = "mineclone:grass";
const INITIAL_HORIZONTAL_RADIUS_CHUNKS: i32 = 5;
const INITIAL_VERTICAL_RADIUS_CHUNKS: i32 = 4;
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
    mut terrain_material_assets: ResMut<Assets<TerrainMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    current_dimension: Res<CurrentDimension>,
    seed: Res<WorldSeed>,
    load_mode: Res<WorldLoadMode>,
    mut save: ResMut<InMemoryWorldSave>,
    dimensions: Res<DimensionRegistry>,
    biomes: Res<BiomeRegistry>,
    blocks: Res<BlockRegistry>,
    fluids: Res<FluidRegistry>,
    existing_world: Option<Res<VoxelWorld>>,
) {
    let fresh_content = if *load_mode == WorldLoadMode::New {
        Some(read_content())
    } else {
        None
    };
    let dimensions_ref = fresh_content
        .as_ref()
        .map_or(&*dimensions, |content| &content.dimensions);
    let biomes_ref = fresh_content
        .as_ref()
        .map_or(&*biomes, |content| &content.biomes);
    let blocks_ref = fresh_content
        .as_ref()
        .map_or(&*blocks, |content| &content.blocks);
    let fluids_ref = fresh_content
        .as_ref()
        .map_or(&*fluids, |content| &content.fluids);
    let dimension = dimensions_ref
        .get(&current_dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", current_dimension.id));
    let grass = blocks_ref
        .get(GRASS_BLOCK_ID)
        .unwrap_or_else(|| panic!("missing block definition: {GRASS_BLOCK_ID}"));
    let biome_field = BiomeField::from_dimension(dimension, biomes_ref, seed.0);
    let (roughness, metallic) = average_terrain_material(dimension, biomes_ref);
    let mut create_material = |texture: &str| {
        terrain_material_assets.add(TerrainMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(asset_server.load(texture.to_owned())),
                perceptual_roughness: roughness,
                metallic,
                ..default()
            },
            extension: TerrainMaterialExtension::default(),
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
    drop(create_material);
    let fluid_materials = FluidMaterials::from_registry(fluids_ref, &mut standard_materials);
    let initial_center = if *load_mode == WorldLoadMode::Load {
        save.player_position()
            .map(|position| {
                let chunk = split_dimension_position(position).chunk;
                IVec3::new(chunk.x, chunk.y.max(0), chunk.z)
            })
            .unwrap_or(IVec3::ZERO)
    } else {
        let surface_y = surface_height(IVec2::ZERO, dimension, biomes_ref, &biome_field);
        IVec3::new(0, surface_y.div_euclid(CHUNK_SIZE as i32), 0)
    };
    let coords = chunk_coords_in_volume(
        initial_center,
        INITIAL_HORIZONTAL_RADIUS_CHUNKS,
        INITIAL_VERTICAL_RADIUS_CHUNKS,
    );

    match *load_mode {
        WorldLoadMode::New => {
            commands.insert_resource(VoxelWorld::default());
            save.begin_new_world(*seed, &current_dimension.id);
        }
        WorldLoadMode::Load => {
            assert!(save.has_world(), "cannot load a world that is not saved in memory");
            assert!(
                existing_world.is_some(),
                "saved world voxel state is missing from memory"
            );
        }
    }

    commands.insert_resource(biome_field);
    commands.insert_resource(terrain_materials);
    commands.insert_resource(fluid_materials);
    commands.insert_resource(WorldLoadingState {
        coords,
        generated: 0,
        screen_rendered: false,
        transition_requested: false,
    });

    if let Some(content) = fresh_content {
        content.insert(&mut commands);
    }
}

pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    current_dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    blocks: Res<BlockRegistry>,
    fluids: Res<FluidRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    terrain_materials: Res<TerrainMaterials>,
    fluid_materials: Res<FluidMaterials>,
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

        if world.has_generated_chunk(coord) {
            assert!(
                world.restore_chunk(coord),
                "generated chunk must be resident or archived: {coord:?}"
            );
        } else {
            let chunk = generate_chunk(
                coord,
                &blocks,
                &fluids,
                dimension,
                &biomes,
                &biome_field,
            );
            world.insert_chunk(coord, chunk);
        }

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
            &terrain_materials,
            &fluid_materials,
        );
        refresh_adjacent_chunk_meshes(
            &mut commands,
            &mut meshes,
            &mut render_pool,
            &world,
            coord,
            &biomes,
            &biome_field,
            &terrain_materials,
            &fluid_materials,
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
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
) -> (f32, f32) {
    let mut roughness = 0.0;
    let mut metallic = 0.0;
    let mut count = 0.0;

    for biome_id in &dimension.biomes {
        let biome = biomes
            .get(biome_id)
            .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));

        if biome.kind != BiomeKind::Surface {
            continue;
        }

        roughness += biome.visuals.terrain_roughness;
        metallic += biome.visuals.terrain_metallic;
        count += 1.0;
    }

    assert!(
        count > 0.0,
        "dimension {} must define at least one surface biome",
        dimension.id
    );

    (roughness / count, metallic / count)
}
