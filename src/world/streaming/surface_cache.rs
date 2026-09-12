use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    world::{
        biome_field::BiomeField,
        generation::sample_generation_columns,
        world_feature_fields::WorldFeatureFields,
    },
};

const SURFACE_CACHE_MARGIN_CHUNKS: i32 = 2;

pub(super) fn cached_surface_range(
    cache: &mut HashMap<IVec2, (i32, i32)>,
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    feature_fields: &WorldFeatureFields,
) -> (i32, i32) {
    *cache.entry(horizontal_chunk).or_insert_with(|| {
        chunk_surface_range(
            horizontal_chunk,
            dimension,
            biomes,
            biome_field,
            feature_fields,
        )
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
    feature_fields: &WorldFeatureFields,
) -> (i32, i32) {
    let columns = feature_fields.generation_columns(horizontal_chunk, || {
        sample_generation_columns(horizontal_chunk, dimension, biomes, biome_field)
    });
    let mut heights = columns.iter().map(|column| column.surface_height);
    let Some(first) = heights.next() else {
        return (1, 1);
    };

    heights.fold((first, first), |(minimum, maximum), height| {
        (minimum.min(height), maximum.max(height))
    })
}
