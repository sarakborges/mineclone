use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    voxel::chunk::CHUNK_SIZE,
    world::{
        biome_field::{BiomeField, BiomeFieldSample},
        terrain::surface_height_from_sample,
    },
};

pub(super) struct GenerationColumnSample<'a> {
    pub surface_height: i32,
    pub surface: BiomeFieldSample<'a>,
}

pub(super) fn sample_generation_columns<'a>(
    chunk_origin: IVec3,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &'a BiomeField,
) -> Vec<GenerationColumnSample<'a>> {
    let mut columns = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.z + local_z as i32;
            let position = IVec2::new(world_x, world_z);
            let surface = biome_field.sample_surface(position.as_vec2() + Vec2::splat(0.5));
            let surface_height = surface_height_from_sample(
                position,
                dimension,
                biomes,
                biome_field.seed(),
                &surface,
            );

            columns.push(GenerationColumnSample {
                surface_height,
                surface,
            });
        }
    }

    columns
}
