mod caves;
mod columns;
mod density;
mod fluids;
mod index;
mod materials;
mod structures;
pub(crate) mod surface_carvers;

use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, block_id::intern_block_id,
        dimension::DimensionDefinition, fluid::FluidRegistry, structure::StructureRegistry,
        structure_set::StructureSetRegistry,
    },
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, VoxelChunk},
        coordinates::chunk_origin,
        texture_rotation::TextureRotation,
    },
    world::{
        biome_field::BiomeField,
        cave_connectivity::CaveConnectivityRegion,
        generation_region::{
            GenerationRegion, generation_region_coord, generation_region_world_bounds,
        },
        hydrology::HydrologySurfaceSample,
        new_world::{WorldGenerationMode, WorldGenerationSettings},
        terrain::{chunk_y_bounds, surface_height, surface_height_from_sample},
        world_feature_fields::WorldFeatureFields,
    },
};

pub(crate) use self::{
    columns::{GenerationColumnSample, sample_generation_columns},
    fluids::authored_surface_fluid_id_for_position,
};
pub(crate) use self::structures::{
    ResolvedSetPiece, located_structure_origins_in_chunk, resolve_set_pieces,
    structure_candidate_anchor, structure_candidate_probe,
    surface_layer_placements,
};
use self::{
    caves::anchored_cave_region,
    density::{DensityPassContext, sample_density_field},
    fluids::{FluidPassContext, rasterize_fluid_pass},
    materials::{MaterialPassContext, rasterize_material_pass},
    structures::rasterize_structures,
};

const LOCAL_EMPTY_HEADROOM_CHUNKS: i32 = 2;

pub(crate) struct ChunkGenerationContext<'a> {
    pub(crate) blocks: &'a BlockRegistry,
    pub(crate) fluids: &'a FluidRegistry,
    pub(crate) dimension: &'a DimensionDefinition,
    pub(crate) biomes: &'a BiomeRegistry,
    pub(crate) structures: &'a StructureRegistry,
    pub(crate) structure_sets: &'a StructureSetRegistry,
    pub(crate) world_generation: WorldGenerationSettings,
    pub(crate) biome_field: &'a BiomeField,
    pub(crate) feature_fields: &'a WorldFeatureFields,
}

impl ChunkGenerationContext<'_> {
    pub(crate) fn region(&self, region_coord: IVec3) -> Arc<GenerationRegion> {
        self.feature_fields
            .region_with_hydrology(region_coord, |hydrology| {
                hydrology.region_from_macro_terrain(region_coord.xz(), |position| {
                    let surface_position = position.floor().as_ivec2();
                    let surface = self
                        .biome_field
                        .sample_surface(surface_position.as_vec2() + Vec2::splat(0.5));
                    let elevation = surface_height_from_sample(
                        surface_position,
                        self.dimension,
                        self.biome_field,
                        &surface,
                    ) as f32;
                    let raw_continentalness =
                        self.biome_field.climate_at(position).continentalness;
                    let ocean_id = self.dimension.hydrology.ocean_biome.as_deref();
                    let ocean_surface_factor = surface
                        .influences
                        .iter()
                        .filter(|influence| Some(influence.id) == ocean_id)
                        .map(|influence| influence.weight)
                        .sum::<f32>()
                        .clamp(0.0, 1.0);
                    let continentalness = 1.0
                        + (raw_continentalness - 1.0) * ocean_surface_factor;
                    let biome_hydrology = self
                        .biome_field
                        .surface_biome_hydrology(surface.identity_surface_index);

                    HydrologySurfaceSample {
                        elevation,
                        continentalness,
                        biome_hydrology,
                    }
                })
            })
    }

    fn anchored_caves(&self, region: &GenerationRegion) -> Option<Arc<CaveConnectivityRegion>> {
        anchored_cave_region(
            region,
            self.biome_field,
            self.biomes,
            self.dimension,
            self.feature_fields,
        )
    }
}

pub(crate) fn generate_chunk(
    chunk_coord: IVec3,
    context: &ChunkGenerationContext<'_>,
) -> VoxelChunk {
    if chunk_coord.y < 0 {
        return VoxelChunk::empty();
    }

    match context.world_generation.mode() {
        WorldGenerationMode::Void => return generate_void_chunk(chunk_coord, context),
        WorldGenerationMode::Flat => return generate_flat_chunk(chunk_coord, context),
        WorldGenerationMode::Normal => {}
    }

    let horizontal_chunk = chunk_coord.xz();
    let structure_top_chunk =
        maximum_structure_top_chunk_for_horizontal_chunk(horizontal_chunk, context);
    let (_, maximum_surface_chunk_y) = chunk_y_bounds(context.dimension, context.biomes);
    if chunk_coord.y > maximum_surface_chunk_y.max(structure_top_chunk)
        && !context.biomes.has_volume_density_modifiers()
    {
        return VoxelChunk::empty();
    }

    let chunk_origin = chunk_origin(chunk_coord);
    let columns = context
        .feature_fields
        .generation_columns(horizontal_chunk, || {
            sample_generation_columns(
                horizontal_chunk,
                context.dimension,
                context.biomes,
                context.biome_field,
            )
        });
    let local_surface_chunk = columns
        .iter()
        .map(|column| column.surface_height)
        .max()
        .unwrap_or(1)
        .max(context.dimension.sea_level)
        .div_euclid(CHUNK_SIZE as i32);

    // Most volume modifiers only carve existing terrain. Avoid constructing a
    // hydrology/cave/volume region for chunks that are well above any local
    // surface, while retaining two full chunks of headroom for high lake water,
    // biome transitions and structures reaching in from neighboring columns.
    if chunk_coord.y
        > local_surface_chunk.max(structure_top_chunk) + LOCAL_EMPTY_HEADROOM_CHUNKS
        && !context.biomes.has_volume_solid_density_modifiers()
    {
        return VoxelChunk::empty();
    }

    let region_coord = generation_region_coord(chunk_coord);
    let region = context.region(region_coord);
    let volume_region = context
        .feature_fields
        .volume_biome_region(region_coord, || {
            let (minimum, maximum) = generation_region_world_bounds(region_coord);
            context
                .biome_field
                .volume_region_in_bounds(minimum, maximum)
        });
    let chunk_minimum = chunk_origin.as_vec3();
    let chunk_maximum = chunk_minimum + Vec3::splat(CHUNK_SIZE as f32);
    let chunk_volume_region = volume_region.restricted_to_bounds(chunk_minimum, chunk_maximum);
    let anchored_caves = context.anchored_caves(region.as_ref());
    let density = sample_density_field(
        chunk_origin,
        columns.as_ref(),
        &DensityPassContext {
            region: region.as_ref(),
            volume_region: &chunk_volume_region,
            anchored_caves: anchored_caves.as_deref(),
            biome_field: context.biome_field,
            biomes: context.biomes,
            dimension: context.dimension,
        },
    );
    let mut chunk = VoxelChunk::empty();

    rasterize_material_pass(
        &mut chunk,
        chunk_origin,
        columns.as_ref(),
        &density,
        &MaterialPassContext {
            blocks: context.blocks,
            biomes: context.biomes,
            dimension: context.dimension,
            biome_field: context.biome_field,
            region: region.as_ref(),
        },
    );
    rasterize_fluid_pass(
        &mut chunk,
        chunk_origin,
        columns.as_ref(),
        &density.values,
        &FluidPassContext {
            fluids: context.fluids,
            biomes: context.biomes,
            biome_field: context.biome_field,
            sea_level: context.dimension.sea_level,
            region: region.as_ref(),
            anchored_caves: anchored_caves.as_deref(),
            underground_water_fluid: &context.dimension.hydrology.water_fluid,
        },
    );
    if context.world_generation.spawn_structures() {
        rasterize_structures(&mut chunk, chunk_origin, context);
    }

    chunk
}

pub(crate) fn flat_surface_height(dimension: &DimensionDefinition) -> i32 {
    dimension.sea_level.max(1)
}

pub(crate) fn generation_surface_height(
    position: IVec2,
    context: &ChunkGenerationContext<'_>,
) -> i32 {
    match context.world_generation.mode() {
        WorldGenerationMode::Normal => surface_height(
            position,
            context.dimension,
            context.biomes,
            context.biome_field,
        ),
        WorldGenerationMode::Flat => flat_surface_height(context.dimension),
        WorldGenerationMode::Void => 1,
    }
}

fn generate_void_chunk(
    chunk_coord: IVec3,
    context: &ChunkGenerationContext<'_>,
) -> VoxelChunk {
    if chunk_coord != IVec3::ZERO {
        return VoxelChunk::empty();
    }

    let block_id = intern_block_id("asteria:stone");
    let block = context
        .blocks
        .get(block_id)
        .unwrap_or_else(|| panic!("missing void spawn block definition: {block_id}"));
    let mut chunk = VoxelChunk::empty();
    let rotation = TextureRotation::for_position(IVec3::new(8, 0, 8), block.rotate_texture.any());
    chunk.edit_initial_blocks(|blocks| {
        blocks.set_block(8, 0, 8, VoxelCell::new(block_id, rotation));
    });
    chunk
}

fn generate_flat_chunk(
    chunk_coord: IVec3,
    context: &ChunkGenerationContext<'_>,
) -> VoxelChunk {
    let surface_height = flat_surface_height(context.dimension);
    let surface_chunk = (surface_height - 1).div_euclid(CHUNK_SIZE as i32);
    let structure_top = maximum_structure_top_chunk_for_horizontal_chunk(chunk_coord.xz(), context);
    if chunk_coord.y > surface_chunk.max(structure_top) {
        return VoxelChunk::empty();
    }

    let origin = chunk_origin(chunk_coord);
    let mut chunk = VoxelChunk::empty();
    if origin.y < surface_height {
        chunk.edit_initial_blocks(|blocks| {
            for local_z in 0..CHUNK_SIZE {
                for local_x in 0..CHUNK_SIZE {
                    let world_x = origin.x + local_x as i32;
                    let world_z = origin.z + local_z as i32;
                    let surface = context
                        .biome_field
                        .sample_surface(Vec2::new(world_x as f32 + 0.5, world_z as f32 + 0.5));
                    let biome = context
                        .biomes
                        .get(surface.primary_id)
                        .unwrap_or_else(|| panic!("missing flat-world biome: {}", surface.primary_id));

                    for local_y in 0..CHUNK_SIZE {
                        let world_y = origin.y + local_y as i32;
                        if world_y >= surface_height {
                            continue;
                        }
                        let depth = (surface_height - world_y - 1) as u32;
                        let block_id = biome
                            .surface_block_at_depth(depth)
                            .unwrap_or_else(|| panic!(
                                "flat-world biome {} has no surface material at depth {depth}",
                                biome.id
                            ));
                        let block_id = intern_block_id(block_id);
                        let block = context
                            .blocks
                            .get(block_id)
                            .unwrap_or_else(|| panic!("missing flat-world block: {block_id}"));
                        let world_position = IVec3::new(world_x, world_y, world_z);
                        let rotation =
                            TextureRotation::for_position(world_position, block.rotate_texture.any());
                        blocks.set_block(
                            local_x,
                            local_y,
                            local_z,
                            VoxelCell::new(block_id, rotation),
                        );
                    }
                }
            }
        });
    }

    if context.world_generation.spawn_structures() {
        rasterize_structures(&mut chunk, origin, context);
    }
    chunk
}

pub(crate) fn maximum_structure_top_chunk_for_horizontal_chunk(
    horizontal_chunk: IVec2,
    context: &ChunkGenerationContext<'_>,
) -> i32 {
    if !context.world_generation.spawn_structures()
        || context.world_generation.mode() == WorldGenerationMode::Void
    {
        return -1;
    }

    let top_y = context.feature_fields.structure_top_y(horizontal_chunk, || {
        self::structures::maximum_potential_structure_top_y_for_chunk(
            horizontal_chunk,
            context,
        )
    });
    top_y.div_euclid(CHUNK_SIZE as i32)
}
