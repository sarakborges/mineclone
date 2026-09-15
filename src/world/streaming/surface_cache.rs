use bevy::{platform::collections::HashMap, prelude::*};

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    voxel::chunk::CHUNK_SIZE,
    world::{
        biome_field::BiomeField, terrain::surface_height,
        world_feature_fields::WorldFeatureFields,
    },
};

const SURFACE_CACHE_MARGIN_CHUNKS: i32 = 2;
const SURFACE_RANGE_GUARD_BLOCKS: i32 = CHUNK_SIZE as i32;

pub(super) fn cached_surface_range(
    cache: &mut HashMap<IVec2, (i32, i32)>,
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    _feature_fields: &WorldFeatureFields,
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
    let last = chunk_size - 1;
    let center = chunk_size / 2;
    let probes = [
        IVec2::new(0, 0),
        IVec2::new(last, 0),
        IVec2::new(0, last),
        IVec2::new(last, last),
        IVec2::new(center, center),
    ];
    let mut minimum = i32::MAX;
    let mut maximum = i32::MIN;

    for offset in probes {
        let height = surface_height(origin + offset, dimension, biomes, biome_field);
        minimum = minimum.min(height);
        maximum = maximum.max(height);
    }

    // Distant streaming only needs a conservative vertical envelope. Five
    // probes plus a full-chunk guard retain narrow ridges/cuts while avoiding
    // the previous 16 expensive biome/terrain samples for every new column.
    (
        (minimum - SURFACE_RANGE_GUARD_BLOCKS).max(1),
        maximum + SURFACE_RANGE_GUARD_BLOCKS,
    )
}
