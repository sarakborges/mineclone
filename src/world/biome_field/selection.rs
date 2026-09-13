use bevy::prelude::*;

use crate::{
    content::biome::{BiomeClimate, BiomeClimateRange, BiomeVerticalRange},
    world::{
        hydrology::ocean_strength,
        macro_climate::{MacroClimateField, MacroClimateSample},
    },
};

use super::{
    BiomeFieldEntry,
    constants::CLIMATE_BLEND_MARGIN,
    spatial::{cell_hash, hash_unit, surface_site_position},
};

const PROXIMITY_SITE_RADIUS: i32 = 1;

pub(super) fn select_surface_biome_index(
    cell: IVec2,
    site: Vec2,
    site_spacing: Vec2,
    biomes: &[BiomeFieldEntry],
    climate_field: &MacroClimateField,
    seed: u64,
    ocean_biome_id: Option<&str>,
    ocean_weight: f32,
) -> usize {
    let mut nearby_biomes = Vec::new();
    let mut near_ocean = ocean_biome_id.is_some()
        && ocean_strength(climate_field.sample(site).continentalness, ocean_weight) > f32::EPSILON;

    for z in -PROXIMITY_SITE_RADIUS..=PROXIMITY_SITE_RADIUS {
        for x in -PROXIMITY_SITE_RADIUS..=PROXIMITY_SITE_RADIUS {
            if x == 0 && z == 0 {
                continue;
            }

            let neighbor_cell = cell + IVec2::new(x, z);
            let neighbor_site = surface_site_position(neighbor_cell, site_spacing, seed);
            nearby_biomes.push(raw_surface_biome_index(
                neighbor_cell,
                neighbor_site,
                biomes,
                climate_field,
                seed,
            ));

            if ocean_biome_id.is_some()
                && ocean_strength(
                    climate_field.sample(neighbor_site).continentalness,
                    ocean_weight,
                ) > f32::EPSILON
            {
                near_ocean = true;
            }
        }
    }

    let climate = climate_field.sample(site);
    let hash = cell_hash(cell, seed);
    select_weighted_biome_index(biomes, climate, hash, |candidate| {
        candidate.is_regional()
            && proximity_allows(
                candidate,
                &nearby_biomes,
                biomes,
                ocean_biome_id,
                near_ocean,
            )
    })
    .unwrap_or_else(|| raw_surface_biome_index(cell, site, biomes, climate_field, seed))
}

fn raw_surface_biome_index(
    cell: IVec2,
    site: Vec2,
    biomes: &[BiomeFieldEntry],
    climate_field: &MacroClimateField,
    seed: u64,
) -> usize {
    let regional = biomes
        .iter()
        .enumerate()
        .filter_map(|(index, biome)| {
            (biome.is_regional() && biome.weight > f32::EPSILON).then_some(index)
        })
        .collect::<Vec<_>>();
    assert!(
        !regional.is_empty(),
        "surface biome field has no active regional biomes"
    );

    let climate = climate_field.sample(site);
    let hash = cell_hash(cell, seed);

    select_weighted_biome_index(biomes, climate, hash, BiomeFieldEntry::is_regional)
        .unwrap_or_else(|| regional[hash as usize % regional.len()])
}

fn proximity_allows(
    candidate: &BiomeFieldEntry,
    nearby_biomes: &[usize],
    biomes: &[BiomeFieldEntry],
    ocean_biome_id: Option<&str>,
    near_ocean: bool,
) -> bool {
    if near_ocean
        && ocean_biome_id.is_some_and(|ocean_id| {
            candidate
                .avoid_near
                .iter()
                .any(|avoided| avoided == ocean_id)
        })
    {
        return false;
    }

    nearby_biomes.iter().all(|&neighbor_index| {
        let neighbor = &biomes[neighbor_index];
        !biomes_conflict(candidate, neighbor)
    })
}

fn biomes_conflict(left: &BiomeFieldEntry, right: &BiomeFieldEntry) -> bool {
    left.avoid_near
        .iter()
        .any(|avoided| avoided == &right.id)
        || right
            .avoid_near
            .iter()
            .any(|avoided| avoided == &left.id)
}

pub(super) fn select_volume_biome_index(
    world_y: f32,
    climate: MacroClimateSample,
    hash: u64,
    biomes: &[BiomeFieldEntry],
) -> Option<usize> {
    select_weighted_biome_index(biomes, climate, hash, |biome| {
        vertical_range_contains(biome.vertical_range, world_y)
    })
}

fn select_weighted_biome_index(
    biomes: &[BiomeFieldEntry],
    climate: MacroClimateSample,
    hash: u64,
    predicate: impl Fn(&BiomeFieldEntry) -> bool,
) -> Option<usize> {
    let eligible = biomes
        .iter()
        .enumerate()
        .filter_map(|(index, biome)| {
            (predicate(biome) && biome.weight > f32::EPSILON).then_some(index)
        })
        .collect::<Vec<_>>();

    if eligible.is_empty() {
        return None;
    }

    let climate_weighted = eligible
        .iter()
        .map(|index| {
            (
                *index,
                biomes[*index].weight * climate_suitability(biomes[*index].climate, climate),
            )
        })
        .collect::<Vec<_>>();

    if let Some(index) = pick_weighted(&climate_weighted, hash.rotate_left(17)) {
        return Some(index);
    }

    let fallback = eligible
        .iter()
        .map(|index| (*index, biomes[*index].weight))
        .collect::<Vec<_>>();

    pick_weighted(&fallback, hash.rotate_left(29))
}

fn pick_weighted(weighted: &[(usize, f32)], hash: u64) -> Option<usize> {
    let total_weight: f32 = weighted.iter().map(|(_, weight)| *weight).sum();
    if total_weight <= f32::EPSILON {
        return None;
    }

    let mut selector = hash_unit(hash) * total_weight;
    for (index, weight) in weighted {
        selector -= *weight;
        if selector <= 0.0 {
            return Some(*index);
        }
    }

    weighted.last().map(|(index, _)| *index)
}

fn climate_suitability(profile: BiomeClimate, climate: MacroClimateSample) -> f32 {
    climate_axis_suitability(profile.temperature, climate.temperature)
        * climate_axis_suitability(profile.humidity, climate.humidity)
        * climate_axis_suitability(profile.continentalness, climate.continentalness)
        * climate_axis_suitability(profile.erosion, climate.erosion)
}

fn climate_axis_suitability(range: Option<BiomeClimateRange>, value: f32) -> f32 {
    let Some(range) = range else {
        return 1.0;
    };

    if (range.min..=range.max).contains(&value) {
        return 1.0;
    }

    let distance = if value < range.min {
        range.min - value
    } else {
        value - range.max
    };

    1.0 - (distance / CLIMATE_BLEND_MARGIN).clamp(0.0, 1.0)
}

fn vertical_range_contains(range: Option<BiomeVerticalRange>, y: f32) -> bool {
    let Some(range) = range else {
        return y >= 0.0;
    };

    y >= range.min && y <= range.max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_ranges_never_include_negative_world_y() {
        assert!(!vertical_range_contains(None, -1.0));
        assert!(!vertical_range_contains(
            Some(BiomeVerticalRange {
                min: 10.0,
                max: 20.0,
            }),
            -1.0,
        ));
    }
}
