mod caves;
pub(super) mod columns;
mod density;
mod features;
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
        structure_set::StructureSetRegistry,
    },
    voxel::chunk::{CHUNK_SIZE, VoxelChunk},
};

use self::{
    caves::anchored_cave_region,
    columns::sample_generation_columns,
    density::sample_density_field,
    features::rasterize_feature_pass,
    fluids::rasterize_fluid_pass,
    materials::{MaterialPassContext, rasterize_material_pass},
};
use super::{
    biome_field::BiomeField,
    cave_connectivity::CaveConnectivityRegion,
    generation_pipeline::{GENERATION_STAGE_ORDER, GenerationStage},
    generation_region::{GenerationRegion, generation_region_coord, generation_region_world_bounds},
    hydrology::HydrologySurfaceSample,
    terrain::{chunk_y_bounds, surface_height_from_sample},
    world_feature_fields::WorldFeatureFields,
};

pub(crate) struct ChunkGenerationContext<'a> {
    pub blocks: &'a BlockRegistry,
    pub fluids: &'a FluidRegistry,
    pub dimension: &'a DimensionDefinition,
    pub biomes: &'a BiomeRegistry,
    pub structures: &'a StructureRegistry,
    pub structure_sets: &'a StructureSetRegistry,
    pub biome_field: &'a BiomeField,
    pub feature_fields: &'a WorldFeatureFields,
}

impl ChunkGenerationContext<'_> {
    fn region(&self, region_coord: IVec3) -> Arc<GenerationRegion> {
        self.feature_fields
            .region_with_hydrology(region_coord, |hydrology| {
                hydrology.region_from_macro_terrain(
                    IVec2::new(region_coord.x, region_coord.z),
                    |position| {
                        let surface_position =
                            IVec2::new(position.x.floor() as i32, position.y.floor() as i32);
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
                        let continentalness =
                            self.biome_field.climate_at(position).continentalness;
                        let surface_biome =
                            self.biomes.get(surface.primary_id).unwrap_or_else(|| {
                                panic!("missing biome definition: {}", surface.primary_id)
                            });

                        HydrologySurfaceSample {
                            elevation,
                            continentalness,
                            biome_hydrology: surface_biome.hydrology,
                        }
                    },
                )
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

pub(crate) fn generate_chunk(coord: IVec3, context: &ChunkGenerationContext<'_>) -> VoxelChunk {
    if coord.y < 0 {
        return VoxelChunk::empty();
    }

    let (_, maximum_surface_chunk_y) = chunk_y_bounds(context.dimension, context.biomes);
    let structure_height = context.structures.max_height_above_anchor();
    let structure_chunk_allowance = if structure_height == 0 {
        0
    } else {
        (structure_height + CHUNK_SIZE as i32 - 1) / CHUNK_SIZE as i32
    };
    if coord.y > maximum_surface_chunk_y + structure_chunk_allowance
        && !context.biomes.has_volume_density_modifiers()
    {
        return VoxelChunk::empty();
    }

    debug_assert_generation_order();

    let chunk_origin = coord * CHUNK_SIZE as i32;
    let horizontal_chunk = IVec2::new(coord.x, coord.z);
    let region_coord = generation_region_coord(coord);
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
        columns.as_slice(),
        region.as_ref(),
        &chunk_volume_region,
        anchored_caves.as_deref(),
        context.biome_field,
        context.biomes,
        context.dimension.sea_level as f32,
    );
    let mut chunk = VoxelChunk::empty();
    let material_context = MaterialPassContext {
        blocks: context.blocks,
        biomes: context.biomes,
        biome_field: context.biome_field,
        region: region.as_ref(),
    };

    rasterize_material_pass(
        &mut chunk,
        chunk_origin,
        columns.as_slice(),
        &density,
        &material_context,
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
    rasterize_feature_pass(&mut chunk, chunk_origin, context);

    chunk
}

fn debug_assert_generation_order() {
    debug_assert_eq!(GENERATION_STAGE_ORDER[0], GenerationStage::SurfaceColumns);
    debug_assert_eq!(GENERATION_STAGE_ORDER[1], GenerationStage::Density);
    debug_assert_eq!(GENERATION_STAGE_ORDER[2], GenerationStage::Materials);
    debug_assert_eq!(GENERATION_STAGE_ORDER[3], GenerationStage::Fluids);
    debug_assert_eq!(GENERATION_STAGE_ORDER[4], GenerationStage::Features);
}
