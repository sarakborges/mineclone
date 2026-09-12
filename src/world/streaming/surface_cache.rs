use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    voxel::chunk::CHUNK_SIZE,
    world::{biome_field::BiomeField, terrain::surface_height},
};

const SURFACE_CACHE_MARGIN_CHUNKS: i32 = 2;
const SURFACE_RANGE_GUARD_BLOCKS: i32 = CHUNK_SIZE as i32 / 2;
const ROUGH_SURFACE_REFINEMENT_THRESHOLD: i32 = CHUNK_SIZE as i32 / 2;

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
    let coarse_offsets = [0, chunk_size / 2, chunk_size - 1];
    let mut minimum = i32::MAX;
    let mut maximum = i32::MIN;

    sample_surface_offsets(
        origin,
        &coarse_offsets,
        dimension,
        biomes,
        biome_field,
        &mut minimum,
        &mut maximum,
    );

    // Flat terrain keeps the cheap 3x3 estimate. Rough chunks get a denser
    // pass so a narrow ridge/cliff between the coarse probes cannot make the
    // streaming selector omit an entire vertical chunk of real terrain.
    if maximum - minimum >= ROUGH_SURFACE_REFINEMENT_THRESHOLD {
        let refined_offsets = [
            0,
            chunk_size / 4,
            chunk_size / 2,
            chunk_size * 3 / 4,
            chunk_size - 1,
        ];
        sample_surface_offsets(
            origin,
            &refined_offsets,
            dimension,
            biomes,
            biome_field,
            &mut minimum,
            &mut maximum,
        );
    }

    // Sampling is intentionally not 16x16 because this runs while rebuilding
    // the streaming queue. Keep half a chunk of conservative headroom in both
    // directions so unsampled local extrema cannot expose a straight void seam.
    (
        (minimum - SURFACE_RANGE_GUARD_BLOCKS).max(1),
        maximum + SURFACE_RANGE_GUARD_BLOCKS,
    )
}

#[allow(clippy::too_many_arguments)]
fn sample_surface_offsets(
    origin: IVec2,
    offsets: &[i32],
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    minimum: &mut i32,
    maximum: &mut i32,
) {
    for &z in offsets {
        for &x in offsets {
            let height = surface_height(origin + IVec2::new(x, z), dimension, biomes, biome_field);
            *minimum = (*minimum).min(height);
            *maximum = (*maximum).max(height);
        }
    }
}
