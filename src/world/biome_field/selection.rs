use bevy::prelude::*;

use crate::{
    content::biome::{BiomeClimate, BiomeClimateRange, BiomeVerticalRange},
    world::macro_climate::{MacroClimateField, MacroClimateSample},
};

use super::{
    BiomeFieldEntry,
    constants::CLIMATE_BLEND_MARGIN,
    spatial::{biome_index, cell_hash, hash_unit},
};

pub(super) fn select_surface_biome_index(
    cell: IVec2,
    site: Vec2,
    biomes: &[BiomeFieldEntry],
    climate_field: &MacroClimateField,
    seed: u64,
) -> usize {
    if biomes
        .iter()
        .all(|biome| climate_is_unrestricted(biome.climate))
    {
        return biome_index(cell, biomes.len(), seed);
    }

    let climate = climate_field.sample(site);
    let hash = cell_hash(cell, seed);
    select_weighted_biome_index(biomes, climate, hash, |_| true)
        .unwrap_or_else(|| biome_index(cell, biomes.len(), seed))
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
        .filter_map(|(index, biome)| predicate(biome).then_some(index))
        .collect::<Vec<_>>();

    if eligible.is_empty() {
        return None;
    }

    if eligible
        .iter()
        .all(|index| climate_is_unrestricted(biomes[*index].climate))
    {
        return Some(eligible[hash as usize % eligible.len()]);
    }

    let weighted = eligible
        .iter()
        .map(|index| (*index, climate_suitability(biomes[*index].climate, climate)))
        .collect::<Vec<_>>();
    let total_weight: f32 = weighted.iter().map(|(_, weight)| *weight).sum();

    if total_weight <= f32::EPSILON {
        return Some(eligible[hash as usize % eligible.len()]);
    }

    let mut selector = hash_unit(hash.rotate_left(17)) * total_weight;
    for (index, weight) in weighted {
        selector -= weight;
        if selector <= 0.0 {
            return Some(index);
        }
    }

    eligible.last().copied()
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

fn climate_is_unrestricted(climate: BiomeClimate) -> bool {
    climate.temperature.is_none()
        && climate.humidity.is_none()
        && climate.continentalness.is_none()
        && climate.erosion.is_none()
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
