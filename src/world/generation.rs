mod caves;
mod columns;
mod density;
mod features;
mod fluids;
mod index;
mod materials;

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, builtin_ids::GRASS_BLOCK_ID,
        dimension::DimensionDefinition, fluid::FluidRegistry,
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
    generation_pipeline::{GENERATION_STAGE_ORDER, GenerationStage},
    generation_region::generation_region_coord,
    hydrology::HydrologySurfaceSample,
    terrain::{chunk_y_bounds, surface_height_from_sample},
    world_feature_fields::WorldFeatureFields,
};

pub(crate) fn generate_chunk(
    coord: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    feature_fields: &WorldFeatureFields,
) -> VoxelChunk {
    if coord.y < 0 {
        return VoxelChunk::empty();
    }

    let (_, maximum_surface_chunk_y) = chunk_y_bounds(dimension, biomes);
    if coord.y > maximum_surface_chunk_y && !biomes.has_volume_density_modifiers() {
        return VoxelChunk::empty();
    }

    assert!(
        blocks.get(GRASS_BLOCK_ID).is_some(),
        "missing block definition: {GRASS_BLOCK_ID}"
    );
    debug_assert_generation_order();

    let chunk_origin = coord * CHUNK_SIZE as i32;
    let region_coord = generation_region_coord(coord);
    let region = feature_fields.region_with_hydrology(region_coord, |hydrology| {
        hydrology.region_from_macro_terrain(
            IVec2::new(region_coord.x, region_coord.z),
            |position| {
                let surface_position =
                    IVec2::new(position.x.floor() as i32, position.y.floor() as i32);
                let surface =
                    biome_field.sample_surface(surface_position.as_vec2() + Vec2::splat(0.5));
                let elevation = surface_height_from_sample(
                    surface_position,
                    dimension,
                    biomes,
                    biome_field.seed(),
                    &surface,
                ) as f32;
                let continentalness = biome_field.climate_at(position).continentalness;
                let surface_biome = biomes
                    .get(surface.primary_id)
                    .unwrap_or_else(|| panic!("missing biome definition: {}", surface.primary_id));

                HydrologySurfaceSample {
                    elevation,
                    continentalness,
                    biome_hydrology: surface_biome.hydrology,
                }
            },
        )
    });
    let anchored_caves = anchored_cave_region(region.as_ref(), biome_field, biomes, feature_fields);
    let columns = sample_generation_columns(chunk_origin, dimension, biomes, biome_field);
    let density = sample_density_field(
        chunk_origin,
        &columns,
        region.as_ref(),
        anchored_caves.as_ref(),
        biomes,
        biome_field,
    );
    let mut chunk = VoxelChunk::empty();
    let material_context = MaterialPassContext {
        blocks,
        biomes,
        biome_field,
        region: region.as_ref(),
    };

    rasterize_material_pass(
        &mut chunk,
        chunk_origin,
        &columns,
        &density,
        &material_context,
    );
    rasterize_fluid_pass(&mut chunk, chunk_origin, &density, fluids, region.as_ref());
    rasterize_feature_pass(&mut chunk, chunk_origin, region.as_ref(), biome_field);

    chunk
}

fn debug_assert_generation_order() {
    debug_assert_eq!(GENERATION_STAGE_ORDER[0], GenerationStage::SurfaceColumns);
    debug_assert_eq!(GENERATION_STAGE_ORDER[1], GenerationStage::Density);
    debug_assert_eq!(GENERATION_STAGE_ORDER[2], GenerationStage::Materials);
    debug_assert_eq!(GENERATION_STAGE_ORDER[3], GenerationStage::Fluids);
    debug_assert_eq!(GENERATION_STAGE_ORDER[4], GenerationStage::Features);
}
