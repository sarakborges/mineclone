pub(super) mod fitting;
pub(super) mod resolver;

use bevy::prelude::*;

use crate::{
    content::biome::{BiomeClimate, BiomeClimateRange, BiomeVerticalRange},
    world::{
        deterministic::mix_hash_u64,
        macro_climate::MacroClimateSample,
    },
};

use super::{
    BiomeField, BiomeFieldEntry,
    constants::{CLIMATE_BLEND_MARGIN, SITE_SEARCH_RADIUS},
    distribution::distribution_strength,
    spatial::{cell_hash, hash_unit, surface_site_position},
};

impl BiomeField {
    pub(super) fn surface_weighted_candidates(
        &self,
        _cell: IVec2,
        site: Vec2,
        climate: MacroClimateSample,
        cell_hash: u64,
    ) -> Vec<WeightedSurfaceCandidate> {
        let mut candidates = self
            .surface_biomes
            .iter()
            .enumerate()
            .filter(|(index, biome)| biome.weight > 0.0 && self.surface_biome_is_enabled(*index))
            .filter_map(|(index, biome)| {
                let distribution = biome
                    .distributions
                    .iter()
                    .copied()
                    .map(|distribution| {
                        distribution_strength(
                            distribution,
                            site,
                            self.seed,
                            biome.id.as_str(),
                        )
                    })
                    .fold(0.0_f32, f32::max);
                if distribution <= 0.0 {
                    return None;
                }

                let climate_weight = climate_weight(climate, biome.climate);
                if climate_weight <= 0.0 {
                    return None;
                }

                Some(WeightedSurfaceCandidate {
                    index,
                    weight: biome.weight * climate_weight * distribution,
                })
            })
            .collect::<Vec<_>>();

        candidates.sort_by(|left, right| {
            let left_hash = candidate_hash(cell_hash, &self.surface_biomes[left.index].id);
            let right_hash = candidate_hash(cell_hash, &self.surface_biomes[right.index].id);
            let left_score = -hash_unit(left_hash).ln() / left.weight.max(f32::MIN_POSITIVE);
            let right_score = -hash_unit(right_hash).ln() / right.weight.max(f32::MIN_POSITIVE);
            left_score
                .total_cmp(&right_score)
                .then_with(|| left.index.cmp(&right.index))
        });
        candidates
    }
}

#[derive(Clone, Copy)]
pub(super) struct WeightedSurfaceCandidate {
    pub(super) index: usize,
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
            (climate_weight > 0.0).then_some(WeightedSurfaceCandidate {
                index,
                weight: biome.weight * climate_weight,
            })
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        let left_hash = candidate_hash(source_hash, &biomes[left.index].id);
        let right_hash = candidate_hash(source_hash, &biomes[right.index].id);
        let left_score = -hash_unit(left_hash).ln() / left.weight.max(f32::MIN_POSITIVE);
        let right_score = -hash_unit(right_hash).ln() / right.weight.max(f32::MIN_POSITIVE);
        left_score
            .total_cmp(&right_score)
            .then_with(|| left.index.cmp(&right.index))
    });

    candidates.first().map(|candidate| candidate.index)
}

pub(super) fn authored_pair_conflicts(left: &BiomeFieldEntry, right: &BiomeFieldEntry) -> bool {
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

pub(super) fn region_claim_hash(cell: IVec2, biome_index: usize, seed: u64) -> u64 {
    let hash = cell_hash(cell, seed)
        ^ (biome_index as u64).wrapping_mul(0x517c_c1b7_2722_0a95)
        ^ 0x94d0_49bb_1331_11eb;
    mix_hash_u64(hash)
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

pub(super) fn surface_sites_share_border(
    left_cell: IVec2,
    left_site: Vec2,
    right_cell: IVec2,
    right_site: Vec2,
    spacing: Vec2,
    seed: u64,
) -> bool {
    let midpoint = (left_site + right_site) * 0.5;
    let pair_distance_squared = midpoint.distance_squared(left_site);

    for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
        for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            let cell = left_cell + IVec2::new(x, z);
            if cell == left_cell || cell == right_cell {
                continue;
            }
            let site = surface_site_position(cell, spacing, seed);
            if midpoint.distance_squared(site) + 0.001 < pair_distance_squared {
                return false;
            }
        }
    }

    true
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

    #[test]
    fn direct_grid_neighbors_share_a_surface_border() {
        let spacing = Vec2::splat(360.0);
        assert!(surface_sites_share_border(
            IVec2::ZERO,
            surface_site_position(IVec2::ZERO, spacing, 42),
            IVec2::X,
            surface_site_position(IVec2::X, spacing, 42),
            spacing,
            42,
        ));
    }

    #[test]
    fn distant_sites_do_not_trigger_adjacency() {
        let spacing = Vec2::splat(360.0);
        let left = IVec2::ZERO;
        let right = IVec2::new(2, 0);
        assert!(!surface_sites_share_border(
            left,
            surface_site_position(left, spacing, 42),
            right,
            surface_site_position(right, spacing, 42),
            spacing,
            42,
        ));
    }

    #[test]
    fn full_ocean_continentalness_range_is_authoritative() {
        let ocean_climate = BiomeClimate {
            continentalness: Some(BiomeClimateRange {
                min: 0.0,
                max: 0.38,
            }),
            ..Default::default()
        };
        let ocean_core = MacroClimateSample {
            temperature: 0.5,
            humidity: 0.5,
            continentalness: 0.2,
            erosion: 0.5,
        };
        let shoreline = MacroClimateSample {
            continentalness: 0.44,
            ..ocean_core
        };
        let inland = MacroClimateSample {
            continentalness: 0.6,
            ..ocean_core
        };

        assert_eq!(climate_weight(ocean_core, ocean_climate), 1.0);
        assert!((0.0..1.0).contains(&climate_weight(shoreline, ocean_climate)));
        assert_eq!(climate_weight(inland, ocean_climate), 0.0);
    }
}
