use bevy::prelude::*;

use crate::{
    content::{
        biome::{BiomeKind, BiomeRegistry},
        dimension::DimensionDefinition,
        read_content,
    },
    player::player_id::LOCAL_PLAYER_ID,
    rendering::terrain_material::TerrainMaterial,
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
const FORCED_SPAWN_SEARCH_RADIUS_STEPS: i32 = 256;
const FORCED_SPAWN_COARSE_STEP_BLOCKS: i32 = 64;
const FORCED_SPAWN_COARSE_RADIUS_STEPS: i32 =
    FORCED_SPAWN_SEARCH_RADIUS_STEPS * SPAWN_SEARCH_STEP_BLOCKS
        / FORCED_SPAWN_COARSE_STEP_BLOCKS;
const FORCED_SPAWN_LOCAL_RADIUS_STEPS: i32 =
    FORCED_SPAWN_COARSE_STEP_BLOCKS / SPAWN_SEARCH_STEP_BLOCKS;

pub(in crate::world) fn begin_world_loading(
    mut commands: Commands,
    mut terrain_material_assets: ResMut<Assets<TerrainMaterial>>,
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
        .validate_references(&dimension.id, biomes, blocks, fluids);

    let forced_spawn_biome = (*persistence.load_mode == WorldLoadMode::New)
        .then(|| persistence.new_world_config.spawn_biome().map(str::to_owned))
        .flatten();
    if let Some(biome_id) = forced_spawn_biome.as_deref() {
        validate_forced_spawn_biome(dimension, biomes, biome_id);
    }

    let biome_field = BiomeField::from_dimension(dimension, biomes, config.seed.0);
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
    let feature_fields = WorldFeatureFields::new(
        config.seed.0,
        dimension.sea_level,
        dimension.hydrology.clone(),
        coast_weight,
        ocean_weight,
    );
    let (roughness, metallic) = average_terrain_material(dimension, biomes);
    let terrain_materials = TerrainMaterials::from_registry(
        blocks,
        &content.asset_server,
        &mut terrain_material_assets,
        roughness,
        metallic,
    );
    let fluid_materials = FluidMaterials::from_registry(fluids, &mut terrain_material_assets);
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
            forced_spawn_biome.as_deref(),
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
    let coords = bootstrap_chunk_coords(initial_center, &config.render_distance);
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
    assert!(
        dimension.biomes.iter().any(|entry| entry.id == biome_id),
        "requested spawn biome is not part of dimension {}: {biome_id}",
        dimension.id
    );
}

fn find_initial_spawn_column(
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    feature_fields: &WorldFeatureFields,
    forced_spawn_biome: Option<&str>,
) -> IVec2 {
    if let Some(biome_id) = forced_spawn_biome {
        return find_forced_spawn_column(
            dimension,
            biomes,
            biome_field,
            feature_fields,
            biome_id,
        )
        .unwrap_or_else(|| {
            panic!(
                "could not find a dry spawn column in biome {biome_id} within {} blocks",
                FORCED_SPAWN_SEARCH_RADIUS_STEPS * SPAWN_SEARCH_STEP_BLOCKS
            )
        });
    }

    find_map_square_rings(
        DEFAULT_SPAWN_COLUMN,
        SPAWN_SEARCH_RADIUS_STEPS,
        SPAWN_SEARCH_STEP_BLOCKS,
        |candidate| {
            (!spawn_column_has_water(candidate, dimension, biomes, biome_field, feature_fields))
                .then_some(candidate)
        },
    )
    .unwrap_or_else(|| {
        panic!(
            "could not find a dry spawn column within {} blocks",
            SPAWN_SEARCH_RADIUS_STEPS * SPAWN_SEARCH_STEP_BLOCKS
        )
    })
}

fn find_forced_spawn_column(
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    feature_fields: &WorldFeatureFields,
    biome_id: &str,
) -> Option<IVec2> {
    let coarse = find_map_square_rings(
        DEFAULT_SPAWN_COLUMN,
        FORCED_SPAWN_COARSE_RADIUS_STEPS,
        FORCED_SPAWN_COARSE_STEP_BLOCKS,
        |anchor| {
            if !spawn_column_matches_biome(anchor, biome_field, biome_id) {
                return None;
            }

            find_map_square_rings(
                anchor,
                FORCED_SPAWN_LOCAL_RADIUS_STEPS,
                SPAWN_SEARCH_STEP_BLOCKS,
                |candidate| {
                    if !forced_spawn_candidate_in_bounds(candidate)
                        || !spawn_column_matches_biome(candidate, biome_field, biome_id)
                    {
                        return None;
                    }

                    (!spawn_column_has_water(
                        candidate,
                        dimension,
                        biomes,
                        biome_field,
                        feature_fields,
                    ))
                    .then_some(candidate)
                },
            )
        },
    );

    coarse.or_else(|| {
        find_map_square_rings(
            DEFAULT_SPAWN_COLUMN,
            FORCED_SPAWN_SEARCH_RADIUS_STEPS,
            SPAWN_SEARCH_STEP_BLOCKS,
            |candidate| {
                if !spawn_column_matches_biome(candidate, biome_field, biome_id) {
                    return None;
                }

                (!spawn_column_has_water(
                    candidate,
                    dimension,
                    biomes,
                    biome_field,
                    feature_fields,
                ))
                .then_some(candidate)
            },
        )
    })
}

fn forced_spawn_candidate_in_bounds(candidate: IVec2) -> bool {
    let max_distance = FORCED_SPAWN_SEARCH_RADIUS_STEPS * SPAWN_SEARCH_STEP_BLOCKS;
    let offset = candidate - DEFAULT_SPAWN_COLUMN;
    offset.x.abs() <= max_distance && offset.y.abs() <= max_distance
}

fn spawn_column_matches_biome(column: IVec2, biome_field: &BiomeField, biome_id: &str) -> bool {
    biome_field
        .sample_surface(column.as_vec2() + Vec2::splat(0.5))
        .primary_id
        == biome_id
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
                biome_hydrology: primary.hydrology,
            }
        })
    });
    let position = column.as_vec2() + Vec2::splat(0.5);

    region.hydrology.water_at(position).is_some()
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
