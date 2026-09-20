use std::sync::Arc;

use arrayvec::ArrayVec;
use bevy::prelude::*;

use crate::{
    content::structure::StructureDefinition,
    voxel::coordinates::chunk_coord_from_world,
    world::{
        biome_field::MAX_SURFACE_INFLUENCES,
        cave_connectivity::CaveConnectivityRegion,
        density_sampling::{DensitySampleContext, sample_density},
        generation_region::{GenerationRegion, generation_region_coord},
        terrain::{surface_height, surface_height_from_sample, terrain_density},
    },
};

use super::super::{
    ChunkGenerationContext,
    caves::anchored_cave_region,
    surface_carvers::{
        SurfaceCarverColumn, SurfaceCarverResolveCache, SurfaceCarverResolveContext,
        resolve_surface_carver_column,
        surface_carver_density_delta,
    },
};

const MAX_STRUCTURE_GROUND_RISE: i32 = 3;
const SURFACE_CARVER_WATER_CLEARANCE: f32 = 12.0;

/// Both world generation and /place use the exact same bottom-voxel footprint
/// and terrain-variation rule. The caller supplies ground samples from either
/// procedural density or the already-generated, possibly edited voxel world.
pub(crate) fn fit_structure_to_ground(
    anchor: IVec2,
    support_offsets: &[IVec2],
    minimum_offset_y: i32,
    max_slope: i32,
    mut ground_at: impl FnMut(IVec2) -> Option<i32>,
) -> Option<i32> {
    let mut minimum_ground_y = i32::MAX;
    let mut maximum_ground_y = i32::MIN;

    for &offset in support_offsets {
        let ground_y = ground_at(anchor + offset)?;
        minimum_ground_y = minimum_ground_y.min(ground_y);
        maximum_ground_y = maximum_ground_y.max(ground_y);
    }

    if minimum_ground_y == i32::MAX || maximum_ground_y - minimum_ground_y > max_slope {
        return None;
    }

    Some(minimum_ground_y - minimum_offset_y)
}

pub(super) fn compute_structure_origin_y(
    anchor: IVec2,
    structure: &StructureDefinition,
    variant_index: usize,
    context: &ChunkGenerationContext<'_>,
) -> Option<i32> {
    let (region, anchored_caves) = structure_support_context(anchor, context);
    fit_structure_to_ground(
        anchor,
        structure.variant_support_offsets(variant_index),
        structure.variant_min_y_offset(variant_index),
        structure.restrictions.max_slope,
        |position| {
            let sample_position = position.as_vec2() + Vec2::splat(0.5);
            if structure.restrictions.requires_dry_ground
                && region.hydrology.water_at(sample_position).is_some()
            {
                return None;
            }
            supported_surface_ground_y(
                position,
                region.as_ref(),
                anchored_caves.as_deref(),
                context,
            )
        },
    )
}

fn structure_support_context(
    anchor: IVec2,
    context: &ChunkGenerationContext<'_>,
) -> (Arc<GenerationRegion>, Option<Arc<CaveConnectivityRegion>>) {
    let surface_y = surface_height(
        anchor,
        context.dimension,
        context.biomes,
        context.biome_field,
    );
    let mut anchor_chunk = chunk_coord_from_world(IVec3::new(anchor.x, surface_y - 1, anchor.y));
    anchor_chunk.y = anchor_chunk.y.max(0);
    let region = context.region(generation_region_coord(anchor_chunk));
    let anchored_caves = anchored_cave_region(
        region.as_ref(),
        context.biome_field,
        context.biomes,
        context.dimension,
        context.feature_fields,
    );

    (region, anchored_caves)
}

fn supported_surface_ground_y(
    position: IVec2,
    region: &GenerationRegion,
    anchored_caves: Option<&CaveConnectivityRegion>,
    context: &ChunkGenerationContext<'_>,
) -> Option<i32> {
    let horizontal = position.as_vec2() + Vec2::splat(0.5);
    let surface = context.biome_field.sample_surface(horizontal);
    let raw_surface_height = surface_height_from_sample(
        position,
        context.dimension,
        context.biome_field,
        &surface,
    );
    let raw_ground_y = raw_surface_height - 1;
    let surface_carver_minimum_y = raw_ground_y as f32 + 0.5;
    let surface_carver_maximum_y = (raw_ground_y + MAX_STRUCTURE_GROUND_RISE) as f32 + 0.5;
    let influences = surface
        .influences
        .iter()
        .map(|influence| (influence.surface_index, influence.weight))
        .collect::<ArrayVec<_, MAX_SURFACE_INFLUENCES>>();
    let surface_carver_allowed = region
        .hydrology
        .water_near(horizontal, SURFACE_CARVER_WATER_CLEARANCE)
        .is_none();
    let mut surface_carvers = SurfaceCarverColumn::default();
    let mut surface_carver_cache = SurfaceCarverResolveCache::default();
    if surface_carver_allowed {
        resolve_surface_carver_column(
            &mut surface_carvers,
            &mut surface_carver_cache,
            horizontal,
            raw_surface_height as f32,
            surface.identity_surface_index,
            &influences,
            &SurfaceCarverResolveContext {
                dimension: context.dimension,
                biomes: context.biomes,
                biome_field: context.biome_field,
                world_seed: context.biome_field.seed(),
                minimum_y: surface_carver_minimum_y,
                maximum_y: surface_carver_maximum_y,
                cave_graph: anchored_caves.map(|caves| &caves.connector_graph),
            },
        );
    }
    let density_context = DensitySampleContext::new(region, anchored_caves, context.biome_field);

    let density_at = |world_y: i32| {
        let sample_position = Vec3::new(horizontal.x, world_y as f32 + 0.5, horizontal.y);
        let base_density = terrain_density(raw_surface_height, world_y);
        let sampled_density = sample_density(base_density, sample_position, None, &density_context);
        let carver_delta = if surface_carver_allowed {
            surface_carver_density_delta(
                sampled_density,
                sample_position,
                raw_surface_height as f32,
                &surface_carvers,
            )
        } else {
            0.0
        };

        sampled_density + carver_delta
    };

    if density_at(raw_ground_y) <= 0.0 {
        return None;
    }

    let mut ground_y = raw_ground_y;
    for candidate_y in (raw_ground_y + 1)..=(raw_ground_y + MAX_STRUCTURE_GROUND_RISE) {
        if density_at(candidate_y) <= 0.0 {
            break;
        }
        ground_y = candidate_y;
    }

    Some(ground_y)
}
