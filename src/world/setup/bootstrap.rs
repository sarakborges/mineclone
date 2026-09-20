use bevy::{prelude::*, render::storage::ShaderBuffer};

use crate::{
    content::{
        biome::{BiomeKind, BiomeRegistry},
        dimension::DimensionDefinition,
        read_content,
    },
    player::player_id::LOCAL_PLAYER_ID,
    rendering::terrain_material::{TerrainLightingBuffer, TerrainMaterial},
    voxel::{
        chunk::CHUNK_SIZE, coordinates::chunk_coord_from_position,
        spatial_search::find_map_square_rings, world::VoxelWorld,
    },
};

use super::{
    WorldLoadingPhase, WorldLoadingState,
    system_params::{WorldBootstrapConfig, WorldBootstrapContent, WorldBootstrapPersistence},
};
use crate::world::{
    WorldLoadMode,
    biome_field::BiomeField,
    chunk_rendering::{FluidMaterials, TerrainMaterials},
    generation_region::generation_region_coord,
    hydrology::HydrologySurfaceSample,
    render_distance::{RenderDistanceSettings, chunk_coords_in_volume},
    terrain::{surface_height, surface_height_from_sample},
    world_feature_fields::WorldFeatureFields,
};

const BOOTSTRAP_HORIZONTAL_RADIUS_CHUNKS: i32 = 4;
const BOOTSTRAP_VERTICAL_RADIUS_CHUNKS: i32 = 2;
const DEFAULT_SPAWN_COLUMN: IVec2 = IVec2::new(8, 8);
const SPAWN_SEARCH_STEP_BLOCKS: i32 = 8;
const SPAWN_SEARCH_RADIUS_STEPS: i32 = 64;

pub(in crate::world) fn begin_world_loading(
    mut commands: Commands,
    mut terrain_material_assets: ResMut<Assets<TerrainMaterial>>,
    mut shader_buffers: ResMut<Assets<ShaderBuffer>>,
    content: WorldBootstrapContent,
    mut config: WorldBootstrapConfig,
    mut persistence: WorldBootstrapPersistence,
) {
    let fresh_content = if *persistence.load_mode == WorldLoadMode::New {
        Some(read_content())
    } else {
        None
    };
    let dimensions = fresh_content
        .as_ref()
        .map_or(&*content.dimensions, |content| &content.dimensions);
    let biomes = fresh_content
        .as_ref()
        .map_or(&*content.biomes, |content| &content.biomes);
    let blocks = fresh_content
        .as_ref()
        .map_or(&*content.blocks, |content| &content.blocks);
    let fluids = fresh_content
        .as_ref()
        .map_or(&*content.fluids, |content| &content.fluids);
    let dimension = dimensions
        .get(&config.current_dimension.id)
        .unwrap_or_else(|| {
            panic!(
                "missing dimension definition: {}",
                config.current_dimension.id
            )
        });

    dimension.validate_biomes(biomes);
    dimension
        .hydrology
        .validate_references(&dimension.id, biomes, fluids);

    let forced_spawn_biome = match *persistence.load_mode {
        WorldLoadMode::New => persistence.new_world_config.spawn_biome().map(str::to_owned),
        WorldLoadMode::Load => persistence.save.spawn_biome().map(str::to_owned),
    };
    let biome_size_multiplier = match *persistence.load_mode {
        WorldLoadMode::New => persistence.new_world_config.biome_size_multiplier(),
        WorldLoadMode::Load => persistence.save.biome_size_multiplier(),
    };
    if let Some(biome_id) = forced_spawn_biome.as_deref() {
        validate_forced_spawn_biome(dimension, biomes, biome_id);
    }

    let mut biome_field = BiomeField::from_dimension(
        dimension,
        biomes,
        config.seed.0,
        biome_size_multiplier,
    );
    if let Some(biome_id) = forced_spawn_biome.as_deref() {
        biome_field.force_surface_biome(
            biome_id,
            DEFAULT_SPAWN_COLUMN.as_vec2() + Vec2::splat(0.5),
        );
    }
    let ocean_weight = dimension
        .hydrology
        .ocean_biome
        .as_deref()
        .map_or(1.0, |biome_id| dimension.biome_weight(biome_id));
    let feature_fields = WorldFeatureFields::new(
        config.seed.0,
        dimension.sea_level,
        dimension.hydrology.clone(),
        ocean_weight,
    );
    let (roughness, metallic) = average_terrain_material(dimension, biomes);
    let terrain_lighting = TerrainLightingBuffer::new(&mut shader_buffers);
    let terrain_materials = TerrainMaterials::from_registry(
        blocks,
        &content.asset_server,
        &mut terrain_material_assets,
        &terrain_lighting,
        roughness,
        metallic,
    );
    let fluid_materials =
        FluidMaterials::from_registry(fluids, &mut terrain_material_assets, &terrain_lighting);
    let spawn_column = if *persistence.load_mode == WorldLoadMode::Load {
        persistence
            .save
            .player_position(LOCAL_PLAYER_ID)
            .map(|position| IVec2::new(position.x.floor() as i32, position.z.floor() as i32))
            .unwrap_or(DEFAULT_SPAWN_COLUMN)
    } else {
        find_initial_spawn_column(
            dimension,
            biomes,
            &biome_field,
            &feature_fields,
            forced_spawn_biome.is_some(),
        )
    };
    let initial_center = if *persistence.load_mode == WorldLoadMode::Load {
        persistence
            .save
            .player_position(LOCAL_PLAYER_ID)
            .map(|position| {
                let chunk = chunk_coord_from_position(position);
                IVec3::new(chunk.x, chunk.y.max(0), chunk.z)
            })
            .unwrap_or_else(|| {
                let surface_y = surface_height(spawn_column, dimension, biomes, &biome_field);
                IVec3::new(
                    spawn_column.x.div_euclid(CHUNK_SIZE as i32),
                    surface_y.div_euclid(CHUNK_SIZE as i32),
                    spawn_column.y.div_euclid(CHUNK_SIZE as i32),
                )
            })
    } else {
        let surface_y = surface_height(spawn_column, dimension, biomes, &biome_field);
        IVec3::new(
            spawn_column.x.div_euclid(CHUNK_SIZE as i32),
            surface_y.div_euclid(CHUNK_SIZE as i32),
            spawn_column.y.div_euclid(CHUNK_SIZE as i32),
        )
    };
    let mut coords = bootstrap_chunk_coords(initial_center, &config.render_distance);
    if *persistence.load_mode == WorldLoadMode::Load {
        let saved_world = persistence
            .existing_world
            .as_ref()
            .expect("loaded world voxel state must exist before bootstrap");
        coords.retain(|coord| saved_world.has_resident_or_persisted_chunk(*coord));
    }
    let bootstrap_chunks = coords
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    feature_fields.retain_for_chunks(&bootstrap_chunks);

    match *persistence.load_mode {
        WorldLoadMode::New => {
            commands.insert_resource(VoxelWorld::default());
            persistence.save.begin_new_world(
                *config.seed,
                &config.current_dimension.id,
                *config.game_rules,
                forced_spawn_biome.as_deref(),
                biome_size_multiplier,
            );
        }
        WorldLoadMode::Load => {
            assert!(
                persistence.save.has_world(),
                "cannot load a world that is not saved in memory"
            );
            assert!(
                persistence.existing_world.is_some(),
                "saved world voxel state is missing from memory"
            );
            *config.game_rules = persistence.save.game_rules();
        }
    }

    commands.insert_resource(biome_field);
    commands.insert_resource(feature_fields);
    commands.insert_resource(terrain_lighting);
    commands.insert_resource(terrain_materials);
    commands.insert_resource(fluid_materials);
    commands.insert_resource(WorldLoadingState {
        coords,
        generation_cursor: 0,
        generated: 0,
        lit: 0,
        mesh_cursor: 0,
        meshed: 0,
        spawn_column,
        fluid_priming: Default::default(),
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

fn validate_forced_spawn_biome(
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_id: &str,
) {
    let biome = biomes
        .get(biome_id)
        .unwrap_or_else(|| panic!("requested spawn biome is missing: {biome_id}"));
    assert!(
        biome.kind == BiomeKind::Surface,
        "requested spawn biome must be a surface biome: {biome_id}"
    );
    let dimension_biome = dimension
        .biomes
        .iter()
        .find(|entry| entry.id == biome_id)
        .unwrap_or_else(|| {
            panic!(
                "requested spawn biome is not part of dimension {}: {biome_id}",
                dimension.id
            )
        });
    assert!(
        dimension_biome.require_near.is_empty(),
        "requested spawn biome cannot be forced alone because it requires an adjacent biome: {biome_id}"
    );
}

fn find_initial_spawn_column(
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    feature_fields: &WorldFeatureFields,
    restrict_to_forced_region: bool,
) -> IVec2 {
    find_map_square_rings(
        DEFAULT_SPAWN_COLUMN,
        SPAWN_SEARCH_RADIUS_STEPS,
        SPAWN_SEARCH_STEP_BLOCKS,
        |candidate| {
            let position = candidate.as_vec2() + Vec2::splat(0.5);
            if restrict_to_forced_region && !biome_field.forced_surface_core_contains(position) {
                return None;
            }

            (!spawn_column_has_water(candidate, dimension, biomes, biome_field, feature_fields))
                .then_some(candidate)
        },
    )
    .unwrap_or_else(|| {
        if restrict_to_forced_region {
            panic!("could not find a dry spawn column inside the forced initial biome region")
        }
        panic!(
            "could not find a dry spawn column within {} blocks",
            SPAWN_SEARCH_RADIUS_STEPS * SPAWN_SEARCH_STEP_BLOCKS
        )
    })
}

fn spawn_column_has_water(
    column: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    feature_fields: &WorldFeatureFields,
) -> bool {
    let chunk_coord = IVec3::new(
        column.x.div_euclid(CHUNK_SIZE as i32),
        0,
        column.y.div_euclid(CHUNK_SIZE as i32),
    );
    let region_coord = generation_region_coord(chunk_coord);
    let region = feature_fields.region_with_hydrology(region_coord, |hydrology| {
        hydrology.region_from_macro_terrain(region_coord.xz(), |position| {
            let surface_position = position.floor().as_ivec2();
            let surface = biome_field.sample_surface(surface_position.as_vec2() + Vec2::splat(0.5));
            let elevation = surface_height_from_sample(
                surface_position,
                dimension,
                biome_field,
                &surface,
            ) as f32;
            let continentalness = biome_field.climate_at(position).continentalness;
            let primary = biomes
                .get(surface.primary_id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", surface.primary_id));

            HydrologySurfaceSample {
                elevation,
                continentalness,
                biome_hydrology: primary.hydrology.rules(),
            }
        })
    });
    let position = column.as_vec2() + Vec2::splat(0.5);
    let surface_height = surface_height(column, dimension, biomes, biome_field) as f32;

    region
        .hydrology
        .supported_water_at(position, surface_height)
        .is_some()
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

        roughness += biome.visuals().terrain_roughness;
        metallic += biome.visuals().terrain_metallic;
        count += 1.0;
    }

    assert!(
        count > 0.0,
        "dimension {} must define at least one surface biome",
        dimension.id
    );

    (roughness / count, metallic / count)
}
