use bevy::prelude::*;

use crate::{
    content::biome::{BiomeClimate, BiomeClimateRange, BiomeVerticalRange},
    world::macro_climate::{MacroClimateField, MacroClimateSample},
};

use super::{
    constants::CLIMATE_BLEND_MARGIN,
    spatial::{biome_index, cell_hash, hash_unit},
    BiomeFieldEntry,
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

pub(super) fn select_tied_volume_index(
    indices: &[usize],
    source_hashes: &[Option<u64>],
    seed: u64,
) -> Option<usize> {
    if indices.is_empty() {
        return None;
    }

    if indices.len() == 1 {
        return Some(indices[0]);
    }

    let mut candidates = indices
        .iter()
        .map(|index| (*index, source_hashes[*index].unwrap_or_default()))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, source_hash)| *source_hash);

    let mut hash = seed ^ 0x6a09_e667_f3bc_c909;
    for (_, source_hash) in &candidates {
        hash ^= source_hash.wrapping_mul(0x9e37_79b1_85eb_ca87);
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
        hash ^= hash >> 33;
    }

    Some(candidates[hash as usize % candidates.len()].0)
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
    use crate::content::biome::{BiomeSize, BiomeSizeAxis};

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

    #[test]
    fn higher_priority_volume_wins_before_random_tiebreak() {
        let size = BiomeSize {
            x: BiomeSizeAxis { min: 1.0, max: 1.0 },
            y: Some(BiomeSizeAxis { min: 1.0, max: 1.0 }),
            z: BiomeSizeAxis { min: 1.0, max: 1.0 },
        };
        let biomes = [
            BiomeFieldEntry {
                id: "low".into(),
                size,
                climate: BiomeClimate::default(),
                vertical_range: None,
                priority: 0,
            },
            BiomeFieldEntry {
                id: "high".into(),
                size,
                climate: BiomeClimate::default(),
                vertical_range: None,
                priority: 5,
            },
        ];
        let weights = [0.9_f32, 0.4_f32];
        let winning_priority = weights
            .iter()
            .enumerate()
            .filter_map(|(index, weight)| (*weight > 0.0).then_some(biomes[index].priority))
            .max()
            .unwrap();
        let tied = weights
            .iter()
            .enumerate()
            .filter_map(|(index, weight)| {
                (*weight > 0.0 && biomes[index].priority == winning_priority).then_some(index)
            })
            .collect::<Vec<_>>();

        assert_eq!(tied, vec![1]);
    }

    #[test]
    fn equal_priority_volume_tiebreak_is_seeded_and_deterministic() {
        let tied = [0_usize, 1, 2];
        let source_hashes = [Some(11_u64), Some(29_u64), Some(47_u64)];
        let first = select_tied_volume_index(&tied, &source_hashes, 12345).unwrap();
        let second = select_tied_volume_index(&tied, &source_hashes, 12345).unwrap();

        assert_eq!(first, second);
        assert!(tied.contains(&first));
    }
}
