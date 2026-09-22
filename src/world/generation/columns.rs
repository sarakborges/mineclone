use bevy::prelude::*;
use smallvec::SmallVec;

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    voxel::chunk::CHUNK_SIZE,
    world::{biome_field::BiomeField, terrain::surface_height_from_sample},
};

pub(crate) struct GenerationColumnSample {
    pub(crate) surface_height: i32,
    pub(crate) identity_surface_index: usize,
    pub(crate) primary_terrain_strength: f32,
    pub(crate) surface_margin_index: Option<usize>,
    pub(super) surface_influences: SmallVec<[(usize, f32); 4]>,
}

pub(crate) fn sample_flat_generation_columns(
    horizontal_chunk: IVec2,
    surface_height: i32,
    biome_field: &BiomeField,
) -> Vec<GenerationColumnSample> {
    let mut columns = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);
    let chunk_origin = horizontal_chunk * CHUNK_SIZE as i32;

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.y + local_z as i32;
            let surface = biome_field.sample_surface(
                Vec2::new(world_x as f32 + 0.5, world_z as f32 + 0.5),
            );
            let primary_terrain_strength = surface
                .influences
                .iter()
                .find(|influence| influence.surface_index == surface.primary_surface_index)
                .map_or(1.0, |influence| influence.terrain_strength);
            let surface_influences = surface
                .influences
                .iter()
                .map(|influence| (influence.surface_index, influence.weight))
                .collect();

            columns.push(GenerationColumnSample {
                surface_height,
                identity_surface_index: surface.primary_surface_index,
                primary_terrain_strength,
                surface_margin_index: None,
                surface_influences,
            });
        }
    }

    columns
}

pub(crate) fn sample_generation_columns(
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
            let surface_height =
                surface_height_from_sample(position, dimension, biome_field, &surface);
            let surface_margin_index = resolved_surface_margin_index(
                position,
                surface_height,
                &surface,
                dimension,
                biomes,
                biome_field,
            );
            let identity_surface_index =
                surface_margin_index.unwrap_or(surface.primary_surface_index);
            let primary_terrain_strength = surface
                .influences
                .iter()
                .find(|influence| influence.surface_index == surface.primary_surface_index)
                .map_or(1.0, |influence| influence.terrain_strength);
            let surface_influences = surface
                .influences
                .iter()
                .map(|influence| (influence.surface_index, influence.weight))
                .collect();

            columns.push(GenerationColumnSample {
                surface_height,
                identity_surface_index,
                primary_terrain_strength,
                surface_margin_index,
                surface_influences,
            });
        }
    }

    columns
}

fn resolved_surface_margin_index(
    position: IVec2,
    surface_height: i32,
    surface: &crate::world::biome_field::BiomeFieldSample<'_>,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> Option<usize> {
    let margin_index = surface.surface_margin_index?;
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

    const NEIGHBORS: [IVec2; 4] = [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y];
    let climbs_steep_surface = NEIGHBORS.into_iter().any(|offset| {
        let neighbor_position = position + offset;
        let neighbor_surface =
            biome_field.sample_surface(neighbor_position.as_vec2() + Vec2::splat(0.5));
        let neighbor_height = surface_height_from_sample(
            neighbor_position,
            dimension,
            biome_field,
            &neighbor_surface,
        );
        (neighbor_height - surface_height).abs() > max_slope
    });

    (!climbs_steep_surface).then_some(margin_index)
}
