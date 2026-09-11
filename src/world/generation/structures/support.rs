use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    content::structure::StructureVoxel,
    voxel::chunk::CHUNK_SIZE,
    world::{
        cave_connectivity::CaveConnectivityRegion,
        density_pipeline::{DensitySampleContext, sample_density},
        generation_region::{GenerationRegion, generation_region_coord},
        terrain::{surface_height, terrain_density},
    },
};

use super::super::{
    ChunkGenerationContext,
    surface_carvers::{resolve_surface_carver_column, surface_carver_density_delta},
};

const MAX_STRUCTURE_GROUND_VARIATION: i32 = 1;
const MAX_STRUCTURE_GROUND_RISE: i32 = 3;
const SURFACE_CARVER_WATER_CLEARANCE: f32 = 12.0;

pub(super) fn compute_structure_origin_y(
    anchor: IVec2,
    voxels: &[StructureVoxel<'_>],
    context: &ChunkGenerationContext<'_>,
) -> Option<i32> {
    let (region, anchored_caves) = structure_support_context(anchor, context);
    let minimum_offset_y = voxels.iter().map(|voxel| voxel.offset.y).min()?;
    let mut minimum_ground_y = i32::MAX;
    let mut maximum_ground_y = i32::MIN;

    for voxel in voxels
        .iter()
        .filter(|voxel| voxel.offset.y == minimum_offset_y)
    {
        let horizontal_offset = IVec2::new(voxel.offset.x, voxel.offset.z);
        let position = anchor + horizontal_offset;
        let sample_position = position.as_vec2() + Vec2::splat(0.5);

        if region.hydrology.water_at(sample_position).is_some() {
            return None;
        }

        let ground_y = supported_surface_ground_y(
            position,
            region.as_ref(),
            anchored_caves.as_deref(),
            context,
        )?;
        minimum_ground_y = minimum_ground_y.min(ground_y);
        maximum_ground_y = maximum_ground_y.max(ground_y);
    }

    if minimum_ground_y == i32::MAX
        || maximum_ground_y - minimum_ground_y > MAX_STRUCTURE_GROUND_VARIATION
    {
        return None;
    }

    Some(minimum_ground_y - minimum_offset_y)
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
    let chunk_size = CHUNK_SIZE as i32;
    let anchor_chunk = IVec3::new(
        anchor.x.div_euclid(chunk_size),
        (surface_y - 1).div_euclid(chunk_size).max(0),
        anchor.y.div_euclid(chunk_size),
    );
    let region = context.region(generation_region_coord(anchor_chunk));
    let anchored_caves = context.anchored_caves(region.as_ref());

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
    let raw_surface_height = surface_height(
        position,
        context.dimension,
        context.biomes,
        context.biome_field,
    );
    let raw_ground_y = raw_surface_height - 1;
    let influences = surface
        .influences
        .iter()
        .map(|influence| {
            (
                context.biome_field.surface_biome_index(influence.id),
                influence.weight,
            )
        })
        .collect::<Vec<_>>();
    let surface_carvers = region
        .hydrology
        .water_near(horizontal, SURFACE_CARVER_WATER_CLEARANCE)
        .is_none()
        .then(|| {
            resolve_surface_carver_column(
                horizontal,
                &influences,
                context.biomes,
                context.biome_field,
                context.biome_field.seed(),
                context.dimension.sea_level as f32,
            )
        });
    let density_context = DensitySampleContext::new(region, anchored_caves, context.biome_field);

    let density_at = |world_y: i32| {
        let sample_position = Vec3::new(horizontal.x, world_y as f32 + 0.5, horizontal.y);
        let base_density = terrain_density(raw_surface_height, world_y);
        let sampled_density = sample_density(base_density, sample_position, None, &density_context);
        let carver_delta = surface_carvers.as_ref().map_or(0.0, |carvers| {
            surface_carver_density_delta(sampled_density, sample_position, carvers)
        });

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
