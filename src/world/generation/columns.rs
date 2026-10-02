use bevy::prelude::*;
use smallvec::SmallVec;

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    voxel::chunk::CHUNK_SIZE,
    world::{
        biome_field::{BiomeField, BiomeFieldSample},
        terrain::surface_height_from_sample,
    },
};

use super::biome_map::{BIOME_MAP_HALO, BiomeMapSample, BiomeMapTile};

const SURFACE_LAYER_MAX_SLOPE: i32 = 1;

pub(crate) struct GenerationColumnSample {
    pub(crate) surface_height: i32,
    pub(crate) identity_surface_index: usize,
    pub(crate) primary_terrain_strength: f32,
    pub(crate) ocean_weight: f32,
    pub(crate) surface_margin_index: Option<usize>,
    pub(crate) steep_surface: bool,
    pub(super) surface_influences: SmallVec<[(usize, f32); 4]>,
}

/// Compatibility entry point for callers that do not own the generation-stage
/// biome map. The streaming generator uses `sample_flat_generation_columns_from_map`.
pub(crate) fn sample_flat_generation_columns(
    horizontal_chunk: IVec2,
    surface_height: i32,
    biome_field: &BiomeField,
) -> Vec<GenerationColumnSample> {
    let biome_map = BiomeMapTile::sample(horizontal_chunk, biome_field);
    sample_flat_generation_columns_from_map(surface_height, &biome_map)
}

pub(super) fn sample_flat_generation_columns_from_map(
    surface_height: i32,
    biome_map: &BiomeMapTile,
) -> Vec<GenerationColumnSample> {
    let mut columns = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let surface = biome_map.sample_at(IVec2::new(local_x as i32, local_z as i32));
            let surface_influences = surface
                .influences
                .iter()
                .map(|influence| (influence.surface_index, influence.weight))
                .collect();

            columns.push(GenerationColumnSample {
                surface_height,
                identity_surface_index: surface.primary_surface_index,
                primary_terrain_strength: surface.primary_terrain_strength(),
                ocean_weight: surface.ocean_weight,
                surface_margin_index: None,
                steep_surface: false,
                surface_influences,
            });
        }
    }

    columns
}

/// Compatibility entry point for direct terrain sampling. Runtime generation
/// builds the biome map first and calls `sample_generation_columns_from_map`.
pub(crate) fn sample_generation_columns(
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> Vec<GenerationColumnSample> {
    let biome_map = BiomeMapTile::sample(horizontal_chunk, biome_field);
    sample_generation_columns_from_map(
        horizontal_chunk,
        dimension,
        biomes,
        biome_field,
        &biome_map,
    )
}

pub(super) fn sample_generation_columns_from_map(
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    biome_map: &BiomeMapTile,
) -> Vec<GenerationColumnSample> {
    let chunk_origin = horizontal_chunk * CHUNK_SIZE as i32;
    let heights = sample_surface_heights(chunk_origin, dimension, biome_field, biome_map);
    let mut columns = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let local = IVec2::new(local_x as i32, local_z as i32);
            let surface = biome_map.sample_at(local);
            let surface_height = heights[BiomeMapTile::sample_index(local)];
            let surface_margin_index = resolved_surface_margin_index(
                local,
                surface_height,
                surface,
                &heights,
                biomes,
                biome_field,
            );
            let identity_surface_index =
                surface_margin_index.unwrap_or(surface.primary_surface_index);
            let surface_influences = surface
                .influences
                .iter()
                .map(|influence| (influence.surface_index, influence.weight))
                .collect();
            let steep_surface = cardinal_neighbors(local).into_iter().any(|neighbor| {
                let neighbor_height = heights[BiomeMapTile::sample_index(neighbor)];
                (neighbor_height - surface_height).abs() > SURFACE_LAYER_MAX_SLOPE
            });

            columns.push(GenerationColumnSample {
                surface_height,
                identity_surface_index,
                primary_terrain_strength: surface.primary_terrain_strength(),
                ocean_weight: surface.ocean_weight,
                surface_margin_index,
                steep_surface,
                surface_influences,
            });
        }
    }

    columns
}

fn sample_surface_heights(
    chunk_origin: IVec2,
    dimension: &DimensionDefinition,
    biome_field: &BiomeField,
    biome_map: &BiomeMapTile,
) -> Vec<i32> {
    let edge = CHUNK_SIZE + BIOME_MAP_HALO as usize * 2;
    let mut heights = Vec::with_capacity(edge * edge);

    for local_z in -BIOME_MAP_HALO..CHUNK_SIZE as i32 + BIOME_MAP_HALO {
        for local_x in -BIOME_MAP_HALO..CHUNK_SIZE as i32 + BIOME_MAP_HALO {
            let local = IVec2::new(local_x, local_z);
            let world_position = chunk_origin + local;
            let surface = biome_map.sample_at(local).as_field_sample(biome_field);
            heights.push(surface_height_from_sample(
                world_position,
                dimension,
                biome_field,
                &surface,
            ));
        }
    }

    heights
}

pub(crate) fn ocean_weight_from_surface(
    surface: &BiomeFieldSample<'_>,
    biome_field: &BiomeField,
) -> f32 {
    let Some(ocean_index) = biome_field.ocean_surface_index() else {
        return 0.0;
    };

    surface
        .influences
        .iter()
        .filter(|influence| influence.surface_index == ocean_index)
        .map(|influence| influence.weight)
        .sum::<f32>()
        .clamp(0.0, 1.0)
}

fn resolved_surface_margin_index(
    local: IVec2,
    surface_height: i32,
    surface: &BiomeMapSample,
    heights: &[i32],
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> Option<usize> {
    let margin_index = surface.surface_margin_index?;

    // Ocean shoreline material must never become a ramp over any biome tagged
    // as mountain. The biome map keeps this authored relationship explicit.
    if biome_field.ocean_surface_index() == Some(margin_index)
        && (surface.primary_surface_index == margin_index
            || surface.influences.iter().any(|influence| {
                influence.weight > f32::EPSILON
                    && biome_field.surface_biome_has_tag(influence.surface_index, "mountain")
            }))
    {
        return None;
    }

    let margin_biome_id = biome_field.surface_biome_id(margin_index);
    let margin = biomes
        .get(margin_biome_id)
        .unwrap_or_else(|| panic!("missing surface margin biome definition: {margin_biome_id}"))
        .surface_margin
        .as_ref()
        .expect("resolved surface margin biome must define surfaceMargin");
    let Some(max_slope) = margin.max_slope else {
        return Some(margin_index);
    };

    let climbs_steep_surface = cardinal_neighbors(local).into_iter().any(|neighbor| {
        let neighbor_height = heights[BiomeMapTile::sample_index(neighbor)];
        (neighbor_height - surface_height).abs() > max_slope
    });

    (!climbs_steep_surface).then_some(margin_index)
}

fn cardinal_neighbors(local: IVec2) -> [IVec2; 4] {
    [
        local + IVec2::X,
        local + IVec2::NEG_X,
        local + IVec2::Y,
        local + IVec2::NEG_Y,
    ]
}
