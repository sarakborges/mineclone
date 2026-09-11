use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    voxel::chunk::CHUNK_SIZE,
    world::{biome_field::BiomeField, terrain::surface_height_from_sample},
};

pub(crate) struct GenerationColumnSample {
    pub(super) surface_height: i32,
    pub(super) surface_influences: Vec<(usize, f32)>,
}

pub(super) fn sample_generation_columns(
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> Vec<GenerationColumnSample> {
    let mut columns = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);
    let chunk_origin = horizontal_chunk * CHUNK_SIZE as i32;

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.y + local_z as i32;
            let position = IVec2::new(world_x, world_z);
            let surface = biome_field.sample_surface(position.as_vec2() + Vec2::splat(0.5));
            let surface_height = surface_height_from_sample(
                position,
                dimension,
                biomes,
                biome_field.seed(),
                &surface,
            );
            let surface_influences = surface
                .influences
                .iter()
                .map(|influence| {
                    (
                        biome_field.surface_biome_index(influence.id),
                        influence.weight,
                    )
                })
                .collect();

            columns.push(GenerationColumnSample {
                surface_height,
                surface_influences,
            });
        }
    }

    columns
}
