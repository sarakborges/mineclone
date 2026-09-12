use bevy::prelude::*;

use crate::{
    content::biome::{BiomeClimate, BiomeClimateRange, BiomeVerticalRange},
    world::macro_climate::{MacroClimateField, MacroClimateSample},
};

use super::{
    BiomeFieldEntry,
    constants::CLIMATE_BLEND_MARGIN,
    spatial::{cell_hash, hash_unit},
};

pub(super) fn select_surface_biome_index(
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
    assert!(!regional.is_empty(), "surface biome field has no active regional biomes");

    let climate = climate_field.sample(site);
    let hash = cell_hash(cell, seed);

    select_weighted_biome_index(biomes, climate, hash, BiomeFieldEntry::is_regional)
        .unwrap_or_else(|| regional[hash as usize % regional.len()])
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
