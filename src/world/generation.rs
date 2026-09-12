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
        coordinates::{chunk_origin, chunks_for_block_extent},
    },
    world::{
        biome_field::BiomeField,
        cave_connectivity::CaveConnectivityRegion,
        generation_region::{GenerationRegion, generation_region_coord, generation_region_world_bounds},
        hydrology::HydrologySurfaceSample,
        terrain::{chunk_y_bounds, surface_height_from_sample},
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
                        self.biomes,
                        self.biome_field.seed(),
                        &surface,
                    ) as f32;
                    let continentalness = self.biome_field.climate_at(position).continentalness;
                    let primary = self
                        .biomes
                        .get(surface.primary_id)
                        .unwrap_or_else(|| panic!("missing biome definition: {}", surface.primary_id));

                    HydrologySurfaceSample {
                        elevation,
                        continentalness,
                        biome_hydrology: primary.hydrology,
                    }
                })
            })
    }

    fn anchored_caves(
        &self,
        region: &GenerationRegion,
    ) -> Option<Arc<CaveConnectivityRegion>> {
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

    let (_, maximum_surface_chunk_y) = chunk_y_bounds(context.dimension, context.biomes);
    if chunk_coord.y
        > maximum_surface_chunk_y + maximum_structure_vertical_chunk_allowance(context.structures)
        && !context.biomes.has_volume_density_modifiers()
    {
        return VoxelChunk::empty();
    }

    let chunk_origin = chunk_origin(chunk_coord);
    let horizontal_chunk = chunk_coord.xz();
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
    let columns = context.feature_fields.generation_columns(horizontal_chunk, || {
        sample_generation_columns(
            horizontal_chunk,
            context.dimension,
            context.biomes,
            context.biome_field,
        )
    });
    let density = sample_density_field(
        chunk_origin,
        columns.as_ref(),
        region.as_ref(),
        &chunk_volume_region,
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
            region: region.as_ref(),
        },
    );
    rasterize_fluid_pass(
        &mut chunk,
        chunk_origin,
        &density.values,
        context.fluids,
        region.as_ref(),
        anchored_caves.as_deref(),
        &context.dimension.hydrology.water_fluid,
    );
    rasterize_structures(&mut chunk, chunk_origin, context);

    chunk
}

pub(crate) fn maximum_structure_vertical_chunk_allowance(
    structures: &StructureRegistry,
) -> i32 {
    chunks_for_block_extent(structures.max_height_above_anchor())
}
