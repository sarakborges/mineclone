use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    world::{
        biome_field::BiomeField,
        generation_region::GenerationRegion,
        hydrology::HydrologyWaterKind,
        math::lerp,
        terrain::surface_height,
    },
};

const OCEAN_CAVE_ENTRANCE_CHANCE: f32 = 0.34;
const OCEAN_CAVE_ENTRANCE_MINIMUM_OFFSET: f32 = 12.0;
const OCEAN_CAVE_ENTRANCE_MAXIMUM_OFFSET: f32 = 26.0;
const OCEAN_CAVE_ENTRANCE_REGION_MARGIN: f32 = 8.0;
const OCEAN_CAVE_MINIMUM_STRENGTH: f32 = 0.35;
const OCEAN_CAVE_MINIMUM_DROP: f32 = 5.0;
const SURFACE_CAVE_ENTRANCE_CHANCE: f32 = 0.58;
const SURFACE_CAVE_ENTRANCE_MINIMUM_OFFSET: f32 = 8.0;
const SURFACE_CAVE_ENTRANCE_MAXIMUM_OFFSET: f32 = 28.0;
const SURFACE_CAVE_ENTRANCE_REGION_MARGIN: f32 = 8.0;
const SURFACE_CAVE_MINIMUM_DROP: f32 = 8.0;

pub(super) fn surface_cave_entrance(
    region: &GenerationRegion,
    anchors: &[Vec3],
    minimum: Vec3,
    maximum: Vec3,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> Option<Vec3> {
    let seed = biome_field.seed() ^ 0x510e_527f_ade6_82d1;
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
            let hash = cave_entrance_hash(anchor, seed);
            if hash_unit(hash.rotate_left(17)) >= SURFACE_CAVE_ENTRANCE_CHANCE {
                return None;
            }

            let horizontal = cave_entrance_horizontal_with_range(
                anchor,
                minimum,
                maximum,
                seed,
                SURFACE_CAVE_ENTRANCE_MINIMUM_OFFSET,
                SURFACE_CAVE_ENTRANCE_MAXIMUM_OFFSET,
                SURFACE_CAVE_ENTRANCE_REGION_MARGIN,
            );
            if region.hydrology.water_at(horizontal).is_some() {
                return None;
            }

            let block_position =
                IVec2::new(horizontal.x.floor() as i32, horizontal.y.floor() as i32);
            let surface_y =
                surface_height(block_position, dimension, biomes, biome_field) as f32 + 0.5;
            let drop = surface_y - anchor.y;
            if drop < SURFACE_CAVE_MINIMUM_DROP {
                return None;
            }

            Some((drop, Vec3::new(horizontal.x, surface_y, horizontal.y)))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|(left_drop, left), (right_drop, right)| {
        left_drop
            .total_cmp(right_drop)
            .then_with(|| left.x.total_cmp(&right.x))
            .then_with(|| left.z.total_cmp(&right.z))
    });

    candidates.first().map(|(_, entrance)| *entrance)
}

pub(super) fn ocean_cave_entrance(
    region: &GenerationRegion,
    anchors: &[Vec3],
    minimum: Vec3,
    maximum: Vec3,
    biome_field: &BiomeField,
) -> Option<Vec3> {
    let seed = biome_field.seed() ^ 0x3c6e_f372_fe94_f82b;
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
            let hash = cave_entrance_hash(anchor, seed);
            if hash_unit(hash.rotate_left(11)) >= OCEAN_CAVE_ENTRANCE_CHANCE {
                return None;
            }

            let horizontal = cave_entrance_horizontal(anchor, minimum, maximum, seed);
            let water = region.hydrology.water_at(horizontal)?;
            if water.kind != HydrologyWaterKind::Ocean
                || water.strength < OCEAN_CAVE_MINIMUM_STRENGTH
            {
                return None;
            }

            let opening_y = (water.bed_level + 0.5).max(1.5);
            let drop = opening_y - anchor.y;
            if drop < OCEAN_CAVE_MINIMUM_DROP {
                return None;
            }

            Some((
                drop,
                water.strength,
                Vec3::new(horizontal.x, opening_y, horizontal.y),
            ))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(
        |(left_drop, left_strength, left), (right_drop, right_strength, right)| {
            left_drop
                .total_cmp(right_drop)
                .then_with(|| right_strength.total_cmp(left_strength))
                .then_with(|| left.x.total_cmp(&right.x))
                .then_with(|| left.z.total_cmp(&right.z))
        },
    );

    candidates.first().map(|(_, _, entrance)| *entrance)
}

fn cave_entrance_horizontal(anchor: Vec3, minimum: Vec3, maximum: Vec3, seed: u64) -> Vec2 {
    cave_entrance_horizontal_with_range(
        anchor,
        minimum,
        maximum,
        seed,
        OCEAN_CAVE_ENTRANCE_MINIMUM_OFFSET,
        OCEAN_CAVE_ENTRANCE_MAXIMUM_OFFSET,
        OCEAN_CAVE_ENTRANCE_REGION_MARGIN,
    )
}

fn cave_entrance_horizontal_with_range(
    anchor: Vec3,
    minimum: Vec3,
    maximum: Vec3,
    seed: u64,
    minimum_offset: f32,
    maximum_offset: f32,
    region_margin: f32,
) -> Vec2 {
    let hash = cave_entrance_hash(anchor, seed);
    let angle = hash_unit(hash) * std::f32::consts::TAU;
    let distance = lerp(
        minimum_offset,
        maximum_offset,
        hash_unit(hash.rotate_left(29)),
    );
    let offset = Vec2::new(angle.cos(), angle.sin()) * distance;
    let horizontal = Vec2::new(anchor.x, anchor.z) + offset;

    Vec2::new(
        horizontal
            .x
            .clamp(minimum.x + region_margin, maximum.x - region_margin),
        horizontal
            .y
            .clamp(minimum.z + region_margin, maximum.z - region_margin),
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_entrance_offset_stays_inside_region_margin() {
        let minimum = Vec3::ZERO;
        let maximum = Vec3::splat(128.0);
        let horizontal = cave_entrance_horizontal_with_range(
            Vec3::new(2.0, 30.0, 2.0),
            minimum,
            maximum,
            42,
            SURFACE_CAVE_ENTRANCE_MINIMUM_OFFSET,
            SURFACE_CAVE_ENTRANCE_MAXIMUM_OFFSET,
            SURFACE_CAVE_ENTRANCE_REGION_MARGIN,
        );

        assert!(horizontal.x >= SURFACE_CAVE_ENTRANCE_REGION_MARGIN);
        assert!(horizontal.y >= SURFACE_CAVE_ENTRANCE_REGION_MARGIN);
        assert!(horizontal.x <= maximum.x - SURFACE_CAVE_ENTRANCE_REGION_MARGIN);
        assert!(horizontal.y <= maximum.z - SURFACE_CAVE_ENTRANCE_REGION_MARGIN);
    }
}
