use bevy::prelude::*;

use crate::{
    content::biome::{BiomeClimate, BiomeClimateRange, BiomeVerticalRange},
    world::{deterministic::mix_hash_u64, macro_climate::MacroClimateSample},
};

use super::{
    BiomeField, BiomeFieldEntry,
    constants::CLIMATE_BLEND_MARGIN,
    distribution::distribution_strength,
    spatial::{cell_hash, hash_unit},
};

const SURFACE_FALLBACK_SOFT_WEIGHT_FLOOR: f32 = 0.05;

impl BiomeField {
    pub(super) fn ranked_surface_biome_indices(&self, cell: IVec2, position: Vec2) -> Vec<usize> {
        let climate = self.climate.sample(position);

        // Ocean's fully matching continentalness core remains authoritative.
        // Frontier growth may create several logical Ocean regions as each one
        // reaches its max size, but land may not puncture the ocean core.
        if let Some(ocean_index) = self.ocean_surface_index
            && self.surface_biome_is_enabled(ocean_index)
        {
            let ocean = &self.surface_biomes[ocean_index];
            if ocean.weight > f32::EPSILON
                && ocean
                    .distributions
                    .iter()
                    .copied()
                    .any(|distribution| distribution.is_regional())
                && climate_weight(climate, ocean.climate) >= 1.0 - f32::EPSILON
            {
                return vec![ocean_index];
            }
        }

        let source_hash = cell_hash(cell, self.seed);
        let mut preferred = Vec::new();
        let mut fallback = Vec::new();

        for (index, biome) in self.surface_biomes.iter().enumerate() {
            if biome.weight <= 0.0 || !self.surface_biome_is_enabled(index) {
                continue;
            }

            let distribution = biome
                .distributions
                .iter()
                .copied()
                .map(|distribution| {
                    distribution_strength(distribution, position, self.seed, biome.id.as_str())
                })
                .fold(0.0_f32, f32::max);
            let climate_weight = climate_weight(climate, biome.climate);

            if distribution > 0.0 && climate_weight > 0.0 {
                preferred.push(WeightedBiomeCandidate {
                    index,
                    weight: biome.weight * climate_weight * distribution,
                });
                continue;
            }

            // Climate and authored distributions decide preference, not whether
            // the territorial map is allowed to exist. If every preferred
            // candidate is rejected later by hard adjacency/size constraints,
            // these candidates keep the frontier solvable without relaxing any
            // authored territorial rule.
            fallback.push(WeightedBiomeCandidate {
                index,
                weight: biome.weight
                    * soft_surface_fallback_weight(climate_weight)
                    * soft_surface_fallback_weight(distribution),
            });
        }

        sort_weighted_candidates(&mut preferred, source_hash, &self.surface_biomes);
        sort_weighted_candidates(&mut fallback, source_hash, &self.surface_biomes);

        preferred
            .into_iter()
            .chain(fallback)
            .map(|candidate| candidate.index)
            .collect()
    }
}

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

pub(super) fn surface_biomes_conflict(left: &BiomeFieldEntry, right: &BiomeFieldEntry) -> bool {
    if left.avoid_near.iter().any(|id| id == &right.id)
        || right.avoid_near.iter().any(|id| id == &left.id)
    {
        return true;
    }

    left.id != right.id
        && left.exclusive_neighbor_group.as_ref().is_some_and(|group| {
            right
                .exclusive_neighbor_group
                .as_ref()
                .is_some_and(|right_group| right_group == group)
        })
}

pub(super) fn surface_requirement_satisfied(
    candidate: &BiomeFieldEntry,
    neighbor_indices: &[usize],
    biomes: &[BiomeFieldEntry],
) -> bool {
    candidate.require_near.is_empty()
        || neighbor_indices.iter().copied().any(|neighbor_index| {
            candidate
                .require_near
                .iter()
                .any(|id| id == &biomes[neighbor_index].id)
        })
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

fn soft_surface_fallback_weight(weight: f32) -> f32 {
    SURFACE_FALLBACK_SOFT_WEIGHT_FLOOR
        + (1.0 - SURFACE_FALLBACK_SOFT_WEIGHT_FLOOR) * weight.clamp(0.0, 1.0)
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
    fn surface_fallback_never_turns_soft_preferences_into_exclusion() {
        assert_eq!(
            soft_surface_fallback_weight(0.0),
            SURFACE_FALLBACK_SOFT_WEIGHT_FLOOR
        );
        assert_eq!(soft_surface_fallback_weight(1.0), 1.0);
        assert!(soft_surface_fallback_weight(0.5) > SURFACE_FALLBACK_SOFT_WEIGHT_FLOOR);
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
