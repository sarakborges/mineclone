use std::sync::OnceLock;

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, biome_density::BiomeDensityModifier, dimension::DimensionDefinition,
    },
    voxel::neighbors::CARDINAL_NEIGHBORS,
    world::{
        biome_field::{BiomeField, VolumeBiomeRegion},
        density_sampling::{DensitySampleContext, sample_density},
        generation_region::GenerationRegion,
        terrain::{surface_height, terrain_density},
    },
};

const CAVERN_ANCHOR_SEARCH_RADIUS: i32 = 16;
const CAVERN_ANCHOR_SEARCH_STEP: i32 = 4;
const CAVERN_ANCHOR_NEIGHBOR_PROBE: f32 = 2.0;
const CAVERN_ANCHOR_MINIMUM_OPEN_NEIGHBORS: usize = 3;

pub(super) fn resolve_open_cavern_anchor(
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

    let open_neighbors = CARDINAL_NEIGHBORS
        .into_iter()
        .filter(|direction| {
            cavern_density_is_open(
                position + direction.as_vec3() * CAVERN_ANCHOR_NEIGHBOR_PROBE,
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
