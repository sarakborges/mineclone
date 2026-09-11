use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    voxel::chunk::CHUNK_SIZE,
    world::{biome_field::BiomeField, terrain::surface_height},
};

const SURFACE_CACHE_MARGIN_CHUNKS: i32 = 2;

pub(super) fn cached_surface_range(
    cache: &mut HashMap<IVec2, (i32, i32)>,
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> (i32, i32) {
    *cache.entry(horizontal_chunk).or_insert_with(|| {
        chunk_surface_range(horizontal_chunk, dimension, biomes, biome_field)
    })
}

pub(super) fn prune_surface_cache(
    cache: &mut HashMap<IVec2, (i32, i32)>,
    center: IVec2,
    preload_radius: i32,
) {
    let retention_radius = preload_radius + SURFACE_CACHE_MARGIN_CHUNKS;
    let retention_radius_squared = retention_radius * retention_radius;
    cache.retain(|coord, _| (*coord - center).length_squared() <= retention_radius_squared);
}

fn chunk_surface_range(
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> (i32, i32) {
    let chunk_size = CHUNK_SIZE as i32;
    let origin = horizontal_chunk * chunk_size;
    let sample_offsets = [0, chunk_size / 2, chunk_size - 1];
    let mut minimum = i32::MAX;
    let mut maximum = i32::MIN;

    for z in sample_offsets {
        for x in sample_offsets {
            let height = surface_height(origin + IVec2::new(x, z), dimension, biomes, biome_field);
            minimum = minimum.min(height);
            maximum = maximum.max(height);
        }
    }

    (minimum, maximum)
}
