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
    voxel::{chunk::VoxelChunk, coordinates::chunks_for_block_extent},
    world::{
        biome_field::BiomeField, generation_region::GenerationRegion,
        world_feature_fields::WorldFeatureFields,
    },
};

use self::{
    caves::anchored_cave_region,
    columns::sample_generation_columns,
    density::sample_density_field,
    fluids::rasterize_fluid_pass,
    materials::{MaterialPassContext, rasterize_material_pass},
    structures::rasterize_structures,
};
pub(crate) use self::columns::GenerationColumnSample;

pub(crate) struct ChunkGenerationContext<'a> {
    pub(crate) blocks: &'a BlockRegistry,
    pub(crate) fluids: &'a FluidRegistry,
    pub(crate) dimension: &'a DimensionDefinition,
    pub(crate) biomes: &'a BiomeRegistry,
    pub(crate) structures: &'a StructureRegistry,
    pub(crate) biome_field: &'a BiomeField,
    pub(crate) feature_fields: &'a WorldFeatureFields,
}

pub(crate) fn generate_chunk(
    chunk_coord: IVec3,
    context: &ChunkGenerationContext<'_>,
) -> VoxelChunk {
    let region = generation_region(chunk_coord, context);
    let volume_region = context.feature_fields.volume_biome_region(region.coord, || {
        let (minimum, maximum) = crate::world::generation_region::generation_region_world_bounds(
            region.coord,
        );
        context
            .biome_field
            .volume_region_in_bounds(minimum, maximum)
    });
    let anchored_caves = anchored_cave_region(
        region.as_ref(),
        context.biome_field,
        context.biomes,
        context.dimension,
        context.feature_fields,
    );
    let horizontal_chunk = chunk_coord.xz();
    let columns = context.feature_fields.generation_columns(horizontal_chunk, || {
        sample_generation_columns(
            horizontal_chunk,
            context.dimension,
            context.biomes,
            context.biome_field,
        )
    });
    let chunk_origin = chunk_coord * crate::voxel::chunk::CHUNK_SIZE as i32;
    let density = sample_density_field(
        chunk_origin,
        columns.as_ref(),
        &region,
        &volume_region,
        anchored_caves.as_deref(),
        context.biome_field,
        context.biomes,
        context.dimension.sea_level as f32,
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
            biome_field: context.biome_field,
            region: &region,
        },
    );
    rasterize_fluid_pass(
        &mut chunk,
        chunk_origin,
        &density.values,
        context.fluids,
        &region,
        anchored_caves.as_deref(),
        &context.dimension.hydrology.water_fluid,
    );
    rasterize_structures(&mut chunk, chunk_origin, context);

    chunk
}

fn generation_region(
    chunk_coord: IVec3,
    context: &ChunkGenerationContext<'_>,
) -> Arc<GenerationRegion> {
    let region_coord = crate::world::generation_region::generation_region_coord(chunk_coord);
    context
        .feature_fields
        .region_with_hydrology(region_coord, |hydrology| {
            hydrology.region_from_macro_terrain(region_coord.xz(), |position| {
                let surface = context.biome_field.sample_surface(position);
                let elevation = crate::world::terrain::surface_height_from_sample(
                    position.floor().as_ivec2(),
                    context.dimension,
                    context.biomes,
                    context.biome_field.seed(),
                    &surface,
                ) as f32;
                let continentalness = context
                    .biome_field
                    .climate_at(position)
                    .continentalness;
                let primary = context
                    .biomes
                    .get(surface.primary_id)
                    .unwrap_or_else(|| panic!("missing biome definition: {}", surface.primary_id));

                crate::world::hydrology::HydrologySurfaceSample {
                    elevation,
                    continentalness,
                    biome_hydrology: primary.hydrology,
                }
            })
        })
}

pub(crate) fn maximum_structure_vertical_chunk_allowance(
    structures: &StructureRegistry,
) -> i32 {
    chunks_for_block_extent(structures.max_height_above_anchor())
}
