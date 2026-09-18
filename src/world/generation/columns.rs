use bevy::prelude::*;
use smallvec::SmallVec;

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    voxel::chunk::CHUNK_SIZE,
    world::{biome_field::BiomeField, terrain::surface_height_from_sample},
};

pub(crate) struct GenerationColumnSample {
    pub(crate) surface_height: i32,
    pub(crate) primary_surface_index: usize,
    pub(crate) identity_surface_index: usize,
    pub(crate) primary_terrain_strength: f32,
    pub(crate) surface_margin_index: Option<usize>,
    pub(super) surface_influences: SmallVec<[(usize, f32); 4]>,
}

pub(crate) fn sample_generation_columns(
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    _biomes: &BiomeRegistry,
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
            let surface_height =
                surface_height_from_sample(position, dimension, biome_field, &surface);
            let primary_surface_index = surface.primary_surface_index;
            let identity_surface_index = surface.identity_surface_index;
            let primary_terrain_strength = surface
                .influences
                .iter()
                .find(|influence| influence.surface_index == primary_surface_index)
                .map_or(1.0, |influence| influence.terrain_strength);
            let surface_margin_index = surface.surface_margin_index;
            let surface_influences = surface
                .influences
                .iter()
                .map(|influence| (influence.surface_index, influence.weight))
                .collect();

            columns.push(GenerationColumnSample {
                surface_height,
                primary_surface_index,
                identity_surface_index,
                primary_terrain_strength,
                surface_margin_index,
                surface_influences,
            });
        }
    }

    columns
}
