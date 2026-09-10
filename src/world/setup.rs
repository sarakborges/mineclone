use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        biome::{BiomeKind, BiomeRegistry},
        block::BlockRegistry,
        dimension::{DimensionDefinition, DimensionRegistry},
        fluid::FluidRegistry,
        read_content,
    },
    rendering::terrain_material::TerrainMaterial,
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
    voxel::{
        chunk::CHUNK_SIZE, coordinates::split_dimension_position,
        lighting::initialize_chunk_lighting, world::VoxelWorld,
    },
};

use super::{
    InMemoryWorldSave, WorldLoadMode, WorldSeed,
    biome_field::BiomeField,
    chunk_loading::ensure_chunk_loaded,
    chunk_rendering::{
        FluidMaterials, TerrainMaterials, refresh_adjacent_chunk_meshes,
        refresh_changed_chunk_meshes, spawn_chunk_mesh,
    },
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    dimension::CurrentDimension,
    render_distance::chunk_coords_in_volume,
    terrain::surface_height,
    world_feature_fields::WorldFeatureFields,
};

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

#[derive(SystemParam)]
pub(super) struct WorldLoadingInputs<'w> {
    asset_server: Res<'w, AssetServer>,
    current_dimension: Res<'w, CurrentDimension>,
    seed: Res<'w, WorldSeed>,
    load_mode: Res<'w, WorldLoadMode>,
    dimensions: Res<'w, DimensionRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    blocks: Res<'w, BlockRegistry>,
    fluids: Res<'w, FluidRegistry>,
    existing_world: Option<Res<'w, VoxelWorld>>,
}

pub fn begin_world_loading(
    mut commands: Commands,
    mut terrain_material_assets: ResMut<Assets<TerrainMaterial>>,
    mut save: ResMut<InMemoryWorldSave>,
    inputs: WorldLoadingInputs,
) {
    let fresh_content = if *inputs.load_mode == WorldLoadMode::New {
        Some(read_content())
    } else {
        None
    };
    let dimensions = fresh_content
        .as_ref()
        .map_or(&*inputs.dimensions, |content| &content.dimensions);
    let biomes = fresh_content
        .as_ref()
        .map_or(&*inputs.biomes, |content| &content.biomes);
    let blocks = fresh_content
        .as_ref()
        .map_or(&*inputs.blocks, |content| &content.blocks);
    let fluids = fresh_content
        .as_ref()
        .map_or(&*inputs.fluids, |content| &content.fluids);
    let dimension = dimensions
        .get(&inputs.current_dimension.id)
        .unwrap_or_else(|| {
            panic!(
                "missing dimension definition: {}",
                inputs.current_dimension.id
            )
        });

    dimension
        .hydrology
        .validate_references(&dimension.id, biomes, blocks, fluids);

    let biome_field = BiomeField::from_dimension(dimension, biomes, inputs.seed.0);
    let (roughness, metallic) = average_terrain_material(dimension, biomes);
    let terrain_materials = TerrainMaterials::from_registry(
        blocks,
        &inputs.asset_server,
        &mut terrain_material_assets,
        roughness,
        metallic,
    );
    let fluid_materials = FluidMaterials::from_registry(fluids, &mut terrain_material_assets);
    let initial_center = if *inputs.load_mode == WorldLoadMode::Load {
        save.player_position()
            .map(|position| {
                let chunk = split_dimension_position(position).chunk;
                IVec3::new(chunk.x, chunk.y.max(0), chunk.z)
            })
            .unwrap_or(IVec3::ZERO)
    } else {
        let surface_y = surface_height(IVec2::ZERO, dimension, biomes, &biome_field);
        IVec3::new(0, surface_y.div_euclid(CHUNK_SIZE as i32), 0)
    };
    let coords = chunk_coords_in_volume(
        initial_center,
        INITIAL_HORIZONTAL_RADIUS_CHUNKS,
        INITIAL_VERTICAL_RADIUS_CHUNKS,
    );

    match *inputs.load_mode {
        WorldLoadMode::New => {
            commands.insert_resource(VoxelWorld::default());
            save.begin_new_world(*inputs.seed, &inputs.current_dimension.id);
        }
        WorldLoadMode::Load => {
            assert!(
                save.has_world(),
                "cannot load a world that is not saved in memory"
            );
            assert!(
                inputs.existing_world.is_some(),
                "saved world voxel state is missing from memory"
            );
        }
    }

    commands.insert_resource(biome_field);
    commands.insert_resource(WorldFeatureFields::new(
        inputs.seed.0,
        dimension.sea_level,
        dimension.hydrology.clone(),
    ));
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
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    mut world: ResMut<VoxelWorld>,
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

    let generation_context = generation.context(&content);

    for _ in 0..INITIAL_CHUNKS_PER_FRAME {
        if loading_state.generated >= loading_state.coords.len() {
            break;
        }

        let coord = loading_state.coords[loading_state.generated];
        ensure_chunk_loaded(&mut world, coord, &generation_context);

        let lighting_changes = initialize_chunk_lighting(
            &mut world,
            coord,
            &content.blocks,
            &content.fluids,
        );
        let chunk = world
            .chunk(coord)
            .unwrap_or_else(|| panic!("generated chunk should exist at {coord:?}"));
        let render_context = content.render_context(
            &world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );

        spawn_chunk_mesh(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            chunk,
            &render_context,
        );
        refresh_adjacent_chunk_meshes(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            &render_context,
        );
        refresh_changed_chunk_meshes(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            lighting_changes,
            &render_context,
        );

        loading_state.generated += 1;
    }

    if loading_state.generated >= loading_state.coords.len() && !loading_state.transition_requested
    {
        loading_state.transition_requested = true;
        transition.request(ScreenTransitionTarget::game(GameState::Gameplay));
    }
}

fn average_terrain_material(dimension: &DimensionDefinition, biomes: &BiomeRegistry) -> (f32, f32) {
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
