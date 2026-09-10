use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, biome_density::BiomeDensityModifier, dimension::DimensionDefinition,
    },
    world::{
        biome_field::BiomeField,
        cave_connectivity::CaveConnectivityRegion,
        generation_region::{GenerationRegion, generation_region_world_bounds},
        terrain::surface_height,
        world_feature_fields::WorldFeatureFields,
    },
};

const CAVE_ENTRANCE_MINIMUM_DEPTH: f32 = 8.0;
const CAVE_ENTRANCE_MINIMUM_OFFSET: f32 = 12.0;
const CAVE_ENTRANCE_MAXIMUM_OFFSET: f32 = 26.0;
const CAVE_ENTRANCE_REGION_MARGIN: f32 = 8.0;

pub(super) fn anchored_cave_region(
    region: &GenerationRegion,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
    dimension: &DimensionDefinition,
    feature_fields: &WorldFeatureFields,
) -> Option<Arc<CaveConnectivityRegion>> {
    feature_fields.cave_region(region.coord, |cave_field| {
        let (minimum, maximum) = generation_region_world_bounds(region.coord);
        let margin = Vec3::splat(cave_field.anchor_search_margin());
        let mut anchors = biome_field
            .volume_anchors_in_bounds(minimum - margin, maximum + margin)
            .into_iter()
            .filter(|anchor| {
                let biome = biomes
                    .get(anchor.id)
                    .unwrap_or_else(|| panic!("missing biome definition: {}", anchor.id));
                matches!(
                    biome.density_modifier,
                    Some(BiomeDensityModifier::Cavern { .. })
                )
            })
            .map(|anchor| anchor.position)
            .collect::<Vec<_>>();

        if region.coord.y == 0
            && let Some(entrance) = surface_cave_entrance(
                region,
                &anchors,
                minimum,
                maximum,
                dimension,
                biomes,
                biome_field,
            )
        {
            anchors.push(entrance);
        }

        (anchors.len() >= 2).then(|| cave_field.region_from_anchors(region.coord, &anchors))
    })
}

fn surface_cave_entrance(
    region: &GenerationRegion,
    anchors: &[Vec3],
    minimum: Vec3,
    maximum: Vec3,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> Option<Vec3> {
    let mut candidates = anchors
        .iter()
        .copied()
        .filter(|anchor| {
            anchor.x >= minimum.x
                && anchor.x < maximum.x
                && anchor.z >= minimum.z
                && anchor.z < maximum.z
        })
        .filter_map(|anchor| {
            let horizontal = cave_entrance_horizontal(anchor, minimum, maximum, biome_field.seed());
            let block_position =
                IVec2::new(horizontal.x.floor() as i32, horizontal.y.floor() as i32);
            let surface_y = surface_height(block_position, dimension, biomes, biome_field) as f32;
            let depth = surface_y - anchor.y;

            if depth < CAVE_ENTRANCE_MINIMUM_DEPTH
                || region.hydrology.water_at(horizontal).is_some()
            {
                return None;
            }

            Some((
                depth,
                Vec3::new(horizontal.x, surface_y + 0.5, horizontal.y),
            ))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|(left_depth, left), (right_depth, right)| {
        left_depth
            .total_cmp(right_depth)
            .then_with(|| left.x.total_cmp(&right.x))
            .then_with(|| left.z.total_cmp(&right.z))
    });

    candidates.first().map(|(_, entrance)| *entrance)
}

fn cave_entrance_horizontal(anchor: Vec3, minimum: Vec3, maximum: Vec3, seed: u64) -> Vec2 {
    let hash = cave_entrance_hash(anchor, seed);
    let angle = hash_unit(hash) * std::f32::consts::TAU;
    let distance = lerp(
        CAVE_ENTRANCE_MINIMUM_OFFSET,
        CAVE_ENTRANCE_MAXIMUM_OFFSET,
        hash_unit(hash.rotate_left(29)),
    );
    let offset = Vec2::new(angle.cos(), angle.sin()) * distance;
    let horizontal = Vec2::new(anchor.x, anchor.z) + offset;

    Vec2::new(
        horizontal.x.clamp(
            minimum.x + CAVE_ENTRANCE_REGION_MARGIN,
            maximum.x - CAVE_ENTRANCE_REGION_MARGIN,
        ),
        horizontal.y.clamp(
            minimum.z + CAVE_ENTRANCE_REGION_MARGIN,
            maximum.z - CAVE_ENTRANCE_REGION_MARGIN,
        ),
    )
}

fn cave_entrance_hash(anchor: Vec3, seed: u64) -> u64 {
    let mut hash = seed ^ 0x243f_6a88_85a3_08d3;

    for component in [anchor.x.to_bits(), anchor.y.to_bits(), anchor.z.to_bits()] {
        hash ^= component as u64;
        hash = hash.wrapping_mul(0x9e37_79b1_85eb_ca87);
        hash ^= hash >> 31;
    }

    hash
}

fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}
