use crate::{
    content::biome::{BiomeClimate, BiomeClimateRange, BiomeVerticalRange},
    world::{deterministic::mix_hash_u64, macro_climate::MacroClimateSample},
};

use super::{BiomeFieldEntry, constants::CLIMATE_BLEND_MARGIN, spatial::hash_unit};

#[derive(Clone, Copy)]
struct WeightedBiomeCandidate {
    index: usize,
    weight: f32,
}

pub(super) fn select_volume_biome_index(
    y: f32,
    climate: MacroClimateSample,
    source_hash: u64,
    biomes: &[BiomeFieldEntry],
) -> Option<usize> {
    let mut candidates = biomes
        .iter()
        .enumerate()
        .filter(|(_, biome)| biome.weight > 0.0 && vertical_range_contains(biome.vertical_range, y))
        .filter_map(|(index, biome)| {
            let climate_weight = climate_weight(climate, biome.climate);
            (climate_weight > 0.0).then_some(WeightedBiomeCandidate {
                index,
                weight: biome.weight * climate_weight,
            })
        })
        .collect::<Vec<_>>();

    sort_weighted_candidates(&mut candidates, source_hash, biomes);
    candidates.first().map(|candidate| candidate.index)
}

pub(super) fn climate_weight(sample: MacroClimateSample, climate: BiomeClimate) -> f32 {
    [
        (sample.temperature, climate.temperature),
        (sample.humidity, climate.humidity),
        (sample.continentalness, climate.continentalness),
        (sample.erosion, climate.erosion),
    ]
    .into_iter()
    .map(|(value, range)| climate_axis_weight(value, range))
    .product()
}

fn sort_weighted_candidates(
    candidates: &mut [WeightedBiomeCandidate],
    source_hash: u64,
    biomes: &[BiomeFieldEntry],
) {
    candidates.sort_by(|left, right| {
        let left_hash = candidate_hash(source_hash, &biomes[left.index].id);
        let right_hash = candidate_hash(source_hash, &biomes[right.index].id);
        let left_score = -hash_unit(left_hash).ln() / left.weight.max(f32::MIN_POSITIVE);
        let right_score = -hash_unit(right_hash).ln() / right.weight.max(f32::MIN_POSITIVE);
        left_score
            .total_cmp(&right_score)
            .then_with(|| left.index.cmp(&right.index))
    });
}

fn climate_axis_weight(value: f32, range: Option<BiomeClimateRange>) -> f32 {
    let Some(range) = range else {
        return 1.0;
    };
    if (range.min..=range.max).contains(&value) {
        return 1.0;
    }
    if value < range.min {
        return ((value - (range.min - CLIMATE_BLEND_MARGIN)) / CLIMATE_BLEND_MARGIN)
            .clamp(0.0, 1.0);
    }
    (((range.max + CLIMATE_BLEND_MARGIN) - value) / CLIMATE_BLEND_MARGIN).clamp(0.0, 1.0)
}

fn vertical_range_contains(range: Option<BiomeVerticalRange>, y: f32) -> bool {
    if y < 0.0 {
        return false;
    }
    range.is_none_or(|range| y >= range.min && y <= range.max)
}

fn candidate_hash(source_hash: u64, biome_id: &str) -> u64 {
    let mut hash = source_hash;
    for byte in biome_id.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    mix_hash_u64(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn climate_weight_is_full_inside_range_and_fades_outside() {
        let range = Some(BiomeClimateRange { min: 0.4, max: 0.6 });
        assert_eq!(climate_axis_weight(0.5, range), 1.0);
        assert!(climate_axis_weight(0.35, range) > 0.0);
        assert_eq!(climate_axis_weight(0.2, range), 0.0);
    }

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
