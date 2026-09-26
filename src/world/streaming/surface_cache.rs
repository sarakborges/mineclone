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
    let offsets = [0, chunk_size / 3, chunk_size * 2 / 3, chunk_size - 1];
    let mut minimum = i32::MAX;
    let mut maximum = i32::MIN;

    for z in offsets {
        for x in offsets {
            let height =
                surface_height(origin + IVec2::new(x, z), dimension, biomes, biome_field);
            minimum = minimum.min(height);
            maximum = maximum.max(height);
        }
    }

    // Mountain peaks, volcano slopes and gorge walls can sit between the
    // corners/center of a 16x16 column. A 4x4 probe lattice keeps the estimate
    // spatially dense enough for those authored frequencies, while a full
    // chunk of guard covers the remaining unsampled variation without paying
    // the complete 16x16 generation-column cost on the streaming thread.
    conservative_surface_range(minimum, maximum, dimension.sea_level)
}

fn conservative_surface_range(minimum: i32, maximum: i32, sea_level: i32) -> (i32, i32) {
    (
        (minimum - SURFACE_RANGE_GUARD_BLOCKS).max(1),
        (maximum + SURFACE_RANGE_GUARD_BLOCKS).max(sea_level),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_guard_and_sea_level_share_one_vertical_ceiling() {
        assert_eq!(conservative_surface_range(90, 96, 80), (74, 112));
        assert_eq!(conservative_surface_range(50, 60, 90), (34, 90));
    }
}
