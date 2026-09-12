use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        biome::{BiomeKind, BiomeRegistry},
        block::BlockRegistry,
        dimension::{DimensionDefinition, DimensionRegistry},
        fluid::FluidRegistry,
        read_content,
    },
    player::player_id::LOCAL_PLAYER_ID,
    rendering::terrain_material::TerrainMaterial,
    voxel::{chunk::CHUNK_SIZE, coordinates::chunk_coord_from_position, world::VoxelWorld},
};

use super::{WorldLoadingPhase, WorldLoadingState};
use crate::world::{
    InMemoryWorldSave, NewWorldConfig, WorldLoadMode, WorldSeed,
    biome_field::BiomeField,
    chunk_rendering::{FluidMaterials, TerrainMaterials},
    dimension::CurrentDimension,
    game_rules::GameRules,
    render_distance::{RenderDistanceSettings, chunk_coords_in_volume},
    terrain::surface_height,
    world_feature_fields::WorldFeatureFields,
};

const BOOTSTRAP_HORIZONTAL_RADIUS_CHUNKS: i32 = 2;
const BOOTSTRAP_VERTICAL_RADIUS_CHUNKS: i32 = 1;

#[derive(SystemParam)]
pub(in crate::world) struct WorldLoadingInputs<'w> {
    asset_server: Res<'w, AssetServer>,
    current_dimension: Res<'w, CurrentDimension>,
    seed: Res<'w, WorldSeed>,
    load_mode: Res<'w, WorldLoadMode>,
    new_world_config: Res<'w, NewWorldConfig>,
    render_distance: Res<'w, RenderDistanceSettings>,
    game_rules: ResMut<'w, GameRules>,
    dimensions: Res<'w, DimensionRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    blocks: Res<'w, BlockRegistry>,
    fluids: Res<'w, FluidRegistry>,
    existing_world: Option<Res<'w, VoxelWorld>>,
}

pub(in crate::world) fn begin_world_loading(
    mut commands: Commands,
    mut terrain_material_assets: ResMut<Assets<TerrainMaterial>>,
    mut save: ResMut<InMemoryWorldSave>,
    mut inputs: WorldLoadingInputs,
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

    dimension.validate_biomes(biomes);
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
        save.player_position(LOCAL_PLAYER_ID)
            .map(|position| {
                let chunk = chunk_coord_from_position(position);
                IVec3::new(chunk.x, chunk.y.max(0), chunk.z)
            })
            .unwrap_or(IVec3::ZERO)
    } else {
        let surface_y = surface_height(IVec2::ZERO, dimension, biomes, &biome_field);
        IVec3::new(0, surface_y.div_euclid(CHUNK_SIZE as i32), 0)
    };
    let coords = bootstrap_chunk_coords(initial_center, &inputs.render_distance);

    match *inputs.load_mode {
        WorldLoadMode::New => {
            *inputs.game_rules = inputs.new_world_config.game_rules();
            commands.insert_resource(VoxelWorld::default());
            save.begin_new_world(
                *inputs.seed,
                &inputs.current_dimension.id,
                *inputs.game_rules,
            );
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
            *inputs.game_rules = save.game_rules();
        }
    }

    let coast_weight = dimension
        .hydrology
        .coast_biome
        .as_deref()
        .map_or(1.0, |biome_id| dimension.biome_weight(biome_id));
    let ocean_weight = dimension
        .hydrology
        .ocean_biome
        .as_deref()
        .map_or(1.0, |biome_id| dimension.biome_weight(biome_id));

    commands.insert_resource(biome_field);
    commands.insert_resource(WorldFeatureFields::new(
        inputs.seed.0,
        dimension.sea_level,
        dimension.hydrology.clone(),
        coast_weight,
        ocean_weight,
    ));
    commands.insert_resource(terrain_materials);
    commands.insert_resource(fluid_materials);
    commands.insert_resource(WorldLoadingState {
        coords,
        generated: 0,
        lit: 0,
        meshed: 0,
        phase: WorldLoadingPhase::Generating,
        screen_rendered: false,
        transition_requested: false,
    });

    if let Some(content) = fresh_content {
        content.insert(&mut commands);
    }
}

fn bootstrap_chunk_coords(center: IVec3, render_distance: &RenderDistanceSettings) -> Vec<IVec3> {
    chunk_coords_in_volume(
        center,
        BOOTSTRAP_HORIZONTAL_RADIUS_CHUNKS.min(render_distance.chunks()),
        BOOTSTRAP_VERTICAL_RADIUS_CHUNKS.min(render_distance.vertical_chunks()),
    )
}

fn average_terrain_material(dimension: &DimensionDefinition, biomes: &BiomeRegistry) -> (f32, f32) {
    let mut roughness = 0.0;
    let mut metallic = 0.0;
    let mut count = 0.0;

    for dimension_biome in &dimension.biomes {
        let biome_id = &dimension_biome.id;
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
