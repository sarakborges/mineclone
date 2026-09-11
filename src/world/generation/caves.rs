use std::sync::{Arc, OnceLock};

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, biome_density::BiomeDensityModifier, dimension::DimensionDefinition,
    },
    world::{
        biome_field::{BiomeField, VolumeBiomeRegion},
        cave_connectivity::CaveConnectivityRegion,
        density_pipeline::{DensitySampleContext, sample_density},
        generation_region::{GenerationRegion, generation_region_world_bounds},
        terrain::{surface_height, terrain_density},
        world_feature_fields::WorldFeatureFields,
    },
};

const CAVE_ENTRANCE_MINIMUM_DEPTH: f32 = 8.0;
const CAVE_ENTRANCE_MINIMUM_OFFSET: f32 = 12.0;
const CAVE_ENTRANCE_MAXIMUM_OFFSET: f32 = 26.0;
const CAVE_ENTRANCE_REGION_MARGIN: f32 = 8.0;
const CAVERN_ANCHOR_SEARCH_RADIUS: i32 = 16;
const CAVERN_ANCHOR_SEARCH_STEP: i32 = 4;
const CAVERN_ANCHOR_NEIGHBOR_PROBE: f32 = 2.0;
const CAVERN_ANCHOR_MINIMUM_OPEN_NEIGHBORS: usize = 3;

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
        let search_minimum = minimum - margin;
        let search_maximum = maximum + margin;
        let volume_region = biome_field.volume_region_in_bounds(search_minimum, search_maximum);
        let mut anchors = biome_field
            .volume_anchors_in_region(&volume_region)
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
            .filter_map(|anchor| {
                resolve_open_cavern_anchor(
                    anchor.position,
                    region,
                    &volume_region,
                    dimension,
                    biomes,
                    biome_field,
                )
            })
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

fn resolve_open_cavern_anchor(
    anchor: Vec3,
    region: &GenerationRegion,
    volume_region: &VolumeBiomeRegion,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> Option<Vec3> {
    cavern_anchor_offsets().iter().copied().find_map(|offset| {
        let candidate = snap_to_voxel_center(anchor + offset.as_vec3());
        cavern_space_is_open(
            candidate,
            region,
            volume_region,
            dimension,
            biomes,
            biome_field,
        )
        .then_some(candidate)
    })
}

fn cavern_anchor_offsets() -> &'static [IVec3] {
    static OFFSETS: OnceLock<Vec<IVec3>> = OnceLock::new();

    OFFSETS
        .get_or_init(|| {
            let mut offsets = Vec::new();

            for y in (-CAVERN_ANCHOR_SEARCH_RADIUS..=CAVERN_ANCHOR_SEARCH_RADIUS)
                .step_by(CAVERN_ANCHOR_SEARCH_STEP as usize)
            {
                for z in (-CAVERN_ANCHOR_SEARCH_RADIUS..=CAVERN_ANCHOR_SEARCH_RADIUS)
                    .step_by(CAVERN_ANCHOR_SEARCH_STEP as usize)
                {
                    for x in (-CAVERN_ANCHOR_SEARCH_RADIUS..=CAVERN_ANCHOR_SEARCH_RADIUS)
                        .step_by(CAVERN_ANCHOR_SEARCH_STEP as usize)
                    {
                        offsets.push(IVec3::new(x, y, z));
                    }
                }
            }

            offsets.sort_by(|left, right| {
                left.length_squared()
                    .cmp(&right.length_squared())
                    .then_with(|| left.y.abs().cmp(&right.y.abs()))
                    .then_with(|| left.x.abs().cmp(&right.x.abs()))
                    .then_with(|| left.z.abs().cmp(&right.z.abs()))
                    .then_with(|| left.x.cmp(&right.x))
                    .then_with(|| left.y.cmp(&right.y))
                    .then_with(|| left.z.cmp(&right.z))
            });
            offsets
        })
        .as_slice()
}

fn cavern_space_is_open(
    position: Vec3,
    region: &GenerationRegion,
    volume_region: &VolumeBiomeRegion,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> bool {
    if !cavern_density_is_open(
        position,
        region,
        volume_region,
        dimension,
        biomes,
        biome_field,
    ) {
        return false;
    }

    let probes = [
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::Z,
        Vec3::NEG_Z,
    ];
    let open_neighbors = probes
        .into_iter()
        .filter(|direction| {
            cavern_density_is_open(
                position + *direction * CAVERN_ANCHOR_NEIGHBOR_PROBE,
                region,
                volume_region,
                dimension,
                biomes,
                biome_field,
            )
        })
        .count();

    open_neighbors >= CAVERN_ANCHOR_MINIMUM_OPEN_NEIGHBORS
}

fn cavern_density_is_open(
    position: Vec3,
    region: &GenerationRegion,
    volume_region: &VolumeBiomeRegion,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> bool {
    if position.y < 0.5 {
        return false;
    }

    let Some(volume) = biome_field.volume_selection_in_region(position, volume_region) else {
        return false;
    };
    let Some((modifier, _)) = biome_field.volume_density_modifier(volume) else {
        return false;
    };
    if !matches!(modifier, BiomeDensityModifier::Cavern { .. }) {
        return false;
    }

    let horizontal = IVec2::new(position.x.floor() as i32, position.z.floor() as i32);
    let surface_y = surface_height(horizontal, dimension, biomes, biome_field);
    let world_y = position.y.floor() as i32;
    let base_density = terrain_density(surface_y, world_y);
    let context = DensitySampleContext::new(region, None, biome_field);

    sample_density(base_density, position, Some(volume), &context) < 0.0
}

fn snap_to_voxel_center(position: Vec3) -> Vec3 {
    Vec3::new(
        position.x.floor() + 0.5,
        position.y.floor() + 0.5,
        position.z.floor() + 0.5,
    )
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
