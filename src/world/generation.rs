mod caves;
mod columns;
mod density;
mod fluids;
mod index;
mod materials;
mod structures;
mod surface_carvers;

use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, dimension::DimensionDefinition,
        fluid::FluidRegistry, structure::StructureRegistry,
    },
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        coordinates::chunk_origin,
    },
    world::{
        biome_field::BiomeField,
        cave_connectivity::CaveConnectivityRegion,
        generation_region::{
            GenerationRegion, generation_region_coord, generation_region_world_bounds,
        },
        hydrology::HydrologySurfaceSample,
        terrain::{chunk_y_bounds, surface_height_from_sample},
        world_feature_fields::WorldFeatureFields,
    },
};

pub(crate) use self::columns::{GenerationColumnSample, sample_generation_columns};
pub(crate) use self::structures::{
    fit_structure_to_ground, located_structure_origins_in_chunk, structure_candidate_anchor,
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
    pub(crate) biome_field: &'a BiomeField,
    pub(crate) feature_fields: &'a WorldFeatureFields,
}

impl ChunkGenerationContext<'_> {
    fn region(&self, region_coord: IVec3) -> Arc<GenerationRegion> {
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

    let horizontal_chunk = chunk_coord.xz();
    let structure_top_chunk = maximum_structure_top_chunk_for_horizontal_chunk(
        horizontal_chunk,
        context.dimension,
        context.biomes,
        context.structures,
        context.biome_field,
        context.feature_fields,
    );
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
    rasterize_structures(&mut chunk, chunk_origin, context);

    chunk
}

pub(crate) fn maximum_structure_top_chunk_for_horizontal_chunk(
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    structures: &StructureRegistry,
    biome_field: &BiomeField,
    feature_fields: &WorldFeatureFields,
) -> i32 {
    let top_y = feature_fields.structure_top_y(horizontal_chunk, || {
        self::structures::maximum_potential_structure_top_y_for_chunk(
            horizontal_chunk,
            dimension,
            biomes,
            structures,
            biome_field,
        )
    });
    top_y.div_euclid(CHUNK_SIZE as i32)
}
