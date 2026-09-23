use bevy::{prelude::*, render::storage::ShaderBuffer};

use crate::{
    content::{
        LoadedContent,
        biome::{BiomeKind, BiomeRegistry},
        block::BlockRegistry,
        dimension::{DimensionDefinition, DimensionRegistry},
        fluid::FluidRegistry,
        layer::LayerRegistry,
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
    InMemoryWorldSave, NewWorldConfig, WorldGenerationMode, WorldGenerationSettings, WorldLoadMode,
    biome_field::BiomeField,
    chunk_rendering::{FluidMaterials, TerrainMaterials},
    generation::authored_surface_fluid_id_for_position,
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

struct BootstrapRegistries<'a> {
    dimensions: &'a DimensionRegistry,
    biomes: &'a BiomeRegistry,
    blocks: &'a BlockRegistry,
    layers: &'a LayerRegistry,
    fluids: &'a FluidRegistry,
}

impl<'a> BootstrapRegistries<'a> {
    fn resolve(
        loaded: &'a WorldBootstrapContent<'_>,
        fresh: Option<&'a LoadedContent>,
    ) -> Self {
        match fresh {
            Some(fresh) => Self {
                dimensions: &fresh.dimensions,
                biomes: &fresh.biomes,
                blocks: &fresh.blocks,
                layers: &fresh.layers,
                fluids: &fresh.fluids,
            },
            None => Self {
                dimensions: &loaded.dimensions,
                biomes: &loaded.biomes,
                blocks: &loaded.blocks,
                layers: &loaded.layers,
                fluids: &loaded.fluids,
            },
        }
    }
}

struct BootstrapGenerationSettings {
    forced_spawn_biome: Option<String>,
    biome_size_multiplier: f32,
    world_generation: WorldGenerationSettings,
}

impl BootstrapGenerationSettings {
    fn resolve(
        load_mode: WorldLoadMode,
        new_world_config: &NewWorldConfig,
        save: &InMemoryWorldSave,
    ) -> Self {
        match load_mode {
            WorldLoadMode::New => Self {
                forced_spawn_biome: new_world_config.spawn_biome().map(str::to_owned),
                biome_size_multiplier: new_world_config.biome_size_multiplier(),
                world_generation: new_world_config.world_generation(),
            },
            WorldLoadMode::Load => Self {
                forced_spawn_biome: save.spawn_biome().map(str::to_owned),
                biome_size_multiplier: save.biome_size_multiplier(),
                world_generation: save.world_generation(),
            },
        }
    }
}

struct BootstrapRenderingContext<'a> {
    dimension: &'a DimensionDefinition,
    biomes: &'a BiomeRegistry,
    blocks: &'a BlockRegistry,
    layers: &'a LayerRegistry,
    fluids: &'a FluidRegistry,
    asset_server: &'a AssetServer,
}

struct BootstrapRenderingResources {
    terrain_lighting: TerrainLightingBuffer,
    terrain_materials: TerrainMaterials,
    fluid_materials: FluidMaterials,
}

impl BootstrapRenderingContext<'_> {
    fn build(
        self,
        images: &mut Assets<Image>,
        material_assets: &mut Assets<TerrainMaterial>,
        shader_buffers: &mut Assets<ShaderBuffer>,
    ) -> BootstrapRenderingResources {
        let (roughness, metallic) = average_terrain_material(self.dimension, self.biomes);
        let terrain_lighting = TerrainLightingBuffer::new(shader_buffers);
        let terrain_materials = TerrainMaterials::from_registry(
            self.blocks,
            self.layers,
            self.asset_server,
            images,
            material_assets,
            &terrain_lighting,
            roughness,
            metallic,
        );
        let fluid_materials = FluidMaterials::from_registry(
            self.fluids,
            material_assets,
            &terrain_lighting,
            terrain_materials.texture_array_handle(),
        );

        BootstrapRenderingResources {
            terrain_lighting,
            terrain_materials,
            fluid_materials,
        }
    }
}

struct BootstrapSpawnContext<'a> {
    load_mode: WorldLoadMode,
    world_generation: WorldGenerationSettings,
    forced_spawn_biome: bool,
    dimension: &'a DimensionDefinition,
    biomes: &'a BiomeRegistry,
    biome_field: &'a BiomeField,
    feature_fields: &'a WorldFeatureFields,
}

struct BootstrapSpawn {
    column: IVec2,
    initial_center: IVec3,
}

impl BootstrapSpawnContext<'_> {
    fn resolve(self, saved_player_position: Option<Vec3>) -> BootstrapSpawn {
        let restored_column = saved_player_position
            .map(spawn_column_from_position)
            .unwrap_or(DEFAULT_SPAWN_COLUMN);
        let column = if self.load_mode == WorldLoadMode::New
            && self.world_generation.mode() == WorldGenerationMode::Normal
        {
            find_initial_spawn_column(
                self.dimension,
                self.biomes,
                self.biome_field,
                self.feature_fields,
                self.forced_spawn_biome && !self.world_generation.single_biome(),
                self.world_generation,
            )
        } else {
            restored_column
        };

        let initial_center = if self.world_generation.mode() == WorldGenerationMode::Void
            && saved_player_position.is_none()
        {
            IVec3::ZERO
        } else if self.load_mode == WorldLoadMode::Load {
            saved_player_position
                .map(restored_player_chunk)
                .unwrap_or_else(|| {
                    spawn_surface_chunk(
                        column,
                        surface_height(
                            column,
                            self.dimension,
                            self.biomes,
                            self.biome_field,
                        ),
                    )
                })
        } else {
            let surface_y = if self.world_generation.mode() == WorldGenerationMode::Flat {
                self.dimension.sea_level.max(1)
            } else {
                surface_height(column, self.dimension, self.biomes, self.biome_field)
            };
            spawn_surface_chunk(column, surface_y)
        };

        BootstrapSpawn {
            column,
            initial_center,
        }
    }
}

pub(in crate::world) fn begin_world_loading(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
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
    let BootstrapRegistries {
        dimensions,
        biomes,
        blocks,
        layers,
        fluids,
    } = BootstrapRegistries::resolve(&content, fresh_content.as_ref());
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

    let BootstrapGenerationSettings {
        forced_spawn_biome,
        biome_size_multiplier,
        world_generation,
    } = BootstrapGenerationSettings::resolve(
        *persistence.load_mode,
        &persistence.new_world_config,
        &persistence.save,
    );
    if let Some(biome_id) = forced_spawn_biome.as_deref() {
        validate_forced_spawn_biome(dimension, biomes, biome_id);
    }

    let mut biome_field = BiomeField::from_dimension(
        dimension,
        biomes,
        config.seed.0,
        biome_size_multiplier,
    );
    biome_field.set_spawn_oceans(world_generation.spawn_oceans());
    if world_generation.single_biome() {
        let biome_id = forced_spawn_biome
            .as_deref()
            .expect("single-biome world requires a selected surface biome");
        biome_field.set_single_surface_biome(biome_id);
    } else if let Some(biome_id) = forced_spawn_biome.as_deref() {
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
    let BootstrapRenderingResources {
        terrain_lighting,
        terrain_materials,
        fluid_materials,
    } = BootstrapRenderingContext {
        dimension,
        biomes,
        blocks,
        layers,
        fluids,
        asset_server: &content.asset_server,
    }
    .build(
        &mut images,
        &mut terrain_material_assets,
        &mut shader_buffers,
    );
    let saved_player_position = persistence.save.player_position(LOCAL_PLAYER_ID);
    let BootstrapSpawn {
        column: spawn_column,
        initial_center,
    } = BootstrapSpawnContext {
        load_mode: *persistence.load_mode,
        world_generation,
        forced_spawn_biome: forced_spawn_biome.is_some(),
        dimension,
        biomes,
        biome_field: &biome_field,
        feature_fields: &feature_fields,
    }
    .resolve(saved_player_position);
    // Saves persist only modified chunks. Untouched terrain is intentionally absent and
    // must be regenerated from the pinned worldgen identity around the restored player.
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
                forced_spawn_biome.as_deref(),
                biome_size_multiplier,
                world_generation,
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

    commands.insert_resource(world_generation);
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
        fluid_settling: Default::default(),
        phase: WorldLoadingPhase::Generating,
        screen_rendered: false,
        transition_requested: false,
    });

    if let Some(content) = fresh_content {
        content.insert(&mut commands);
    }
}

fn spawn_column_from_position(position: Vec3) -> IVec2 {
    IVec2::new(position.x.floor() as i32, position.z.floor() as i32)
}

fn restored_player_chunk(position: Vec3) -> IVec3 {
    let chunk = chunk_coord_from_position(position);
    IVec3::new(chunk.x, chunk.y.max(0), chunk.z)
}

fn spawn_surface_chunk(column: IVec2, surface_y: i32) -> IVec3 {
    IVec3::new(
        column.x.div_euclid(CHUNK_SIZE as i32),
        surface_y.div_euclid(CHUNK_SIZE as i32),
        column.y.div_euclid(CHUNK_SIZE as i32),
    )
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
    world_generation: WorldGenerationSettings,
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

            (!spawn_column_has_surface_fluid(
                candidate,
                dimension,
                biomes,
                biome_field,
                feature_fields,
                world_generation,
            ))
                .then_some(candidate)
        },
    )
    .unwrap_or_else(|| {
        if restrict_to_forced_region {
            panic!("could not find a fluid-free spawn column inside the forced initial biome region")
        }
        panic!(
            "could not find a fluid-free spawn column within {} blocks",
            SPAWN_SEARCH_RADIUS_STEPS * SPAWN_SEARCH_STEP_BLOCKS
        )
    })
}

fn spawn_column_has_surface_fluid(
    column: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    feature_fields: &WorldFeatureFields,
    world_generation: WorldGenerationSettings,
) -> bool {
    if authored_surface_fluid_id_for_position(column, dimension, biomes, biome_field).is_some() {
        return true;
    }

    let chunk_coord = IVec3::new(
        column.x.div_euclid(CHUNK_SIZE as i32),
        0,
        column.y.div_euclid(CHUNK_SIZE as i32),
    );
    let region_coord = generation_region_coord(chunk_coord);
    let region = feature_fields.region_with_hydrology(region_coord, |hydrology| {
        hydrology.region_from_macro_terrain(
            region_coord.xz(),
            world_generation.spawn_rivers(),
            world_generation.spawn_lakes(),
            world_generation.spawn_oceans(),
            |position| {
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
        },
        )
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
