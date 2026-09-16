use bevy::prelude::*;

use crate::{
    content::biome::{BiomeClimate, BiomeClimateRange, BiomeVerticalRange},
    world::{hydrology::ocean_strength, macro_climate::MacroClimateSample},
};

use super::{
    BiomeField, BiomeFieldEntry,
    constants::CLIMATE_BLEND_MARGIN,
    spatial::{cell_hash, hash_unit, surface_site_position},
};

const PROXIMITY_SITE_RADIUS: i32 = 1;
const PROXIMITY_NEIGHBOR_COUNT: usize =
    ((PROXIMITY_SITE_RADIUS * 2 + 1) * (PROXIMITY_SITE_RADIUS * 2 + 1) - 1) as usize;
const DOMINANT_NEIGHBOR_LIMIT: usize = 5;

impl BiomeField {
    pub(super) fn select_surface_biome_index(&self, cell: IVec2, site: Vec2) -> usize {
        // The neighborhood is fixed at eight sites. A stack array avoids a
        // heap allocation whenever an uncached biome site is first resolved.
        let mut nearby_biomes = [0; PROXIMITY_NEIGHBOR_COUNT];
        let mut nearby_count = 0;
        let mut near_ocean = self.ocean_biome_id.is_some()
            && ocean_strength(self.climate.sample(site).continentalness, self.ocean_weight)
                > f32::EPSILON;

        for z in -PROXIMITY_SITE_RADIUS..=PROXIMITY_SITE_RADIUS {
            for x in -PROXIMITY_SITE_RADIUS..=PROXIMITY_SITE_RADIUS {
                if x == 0 && z == 0 {
                    continue;
                }

                let neighbor_cell = cell + IVec2::new(x, z);
                let neighbor_site =
                    surface_site_position(neighbor_cell, self.surface_site_spacing, self.seed);
                nearby_biomes[nearby_count] =
                    self.raw_surface_biome_index(neighbor_cell, neighbor_site);
                nearby_count += 1;

                if self.ocean_biome_id.is_some()
                    && ocean_strength(
                        self.climate.sample(neighbor_site).continentalness,
                        self.ocean_weight,
                    ) > f32::EPSILON
                {
                    near_ocean = true;
                }
            }
        }
        debug_assert_eq!(nearby_count, nearby_biomes.len());

        let dominant_neighbor = dominant_neighbor_biome(&nearby_biomes);
        let climate = self.climate.sample(site);
        let hash = cell_hash(cell, self.seed);
        select_weighted_biome_index(&self.surface_biomes, climate, hash, |candidate| {
            candidate.is_regional()
                && dominant_neighbor
                    .is_none_or(|index| candidate.id != self.surface_biomes[index].id)
                && proximity_allows(
                    candidate,
                    &nearby_biomes,
                    &self.surface_biomes,
                    self.ocean_biome_id.as_deref(),
                    near_ocean,
                )
        })
        .unwrap_or_else(|| self.raw_surface_biome_index(cell, site))
    }

    fn raw_surface_biome_index(&self, cell: IVec2, site: Vec2) -> usize {
        let regional_count = self
            .surface_biomes
            .iter()
            .filter(|biome| biome.is_regional() && biome.weight > f32::EPSILON)
            .count();
        assert!(
            regional_count > 0,
            "surface biome field has no active regional biomes"
        );

        let climate = self.climate.sample(site);
        let hash = cell_hash(cell, self.seed);

        select_weighted_biome_index(
            &self.surface_biomes,
            climate,
            hash,
            BiomeFieldEntry::is_regional,
        )
        .unwrap_or_else(|| {
            self.surface_biomes
                .iter()
                .enumerate()
                .filter(|(_, biome)| biome.is_regional() && biome.weight > f32::EPSILON)
                .nth(hash as usize % regional_count)
                .expect("active regional biome fallback must exist")
                .0
        })
    }
}

fn dominant_neighbor_biome(nearby_biomes: &[usize]) -> Option<usize> {
    let mut dominant = None;
    let mut dominant_count = 0;

    for &candidate in nearby_biomes {
        let count = nearby_biomes
            .iter()
            .filter(|&&neighbor| neighbor == candidate)
            .count();
        if count >= DOMINANT_NEIGHBOR_LIMIT && count > dominant_count {
            dominant = Some(candidate);
            dominant_count = count;
        }
    }

    dominant
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
    left.avoid_near.iter().any(|avoided| avoided == &right.id)
        || right.avoid_near.iter().any(|avoided| avoided == &left.id)
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
    // Reuse one allocation for the climate-weighted draw and the raw-weight
    // fallback. The prior pipeline collected eligible indices, climate pairs,
    // and fallback pairs into three separate vectors per selection.
    let mut weighted = Vec::with_capacity(biomes.len());
    for (index, biome) in biomes.iter().enumerate() {
        if predicate(biome) && biome.weight > f32::EPSILON {
            weighted.push((
                index,
                biome.weight * climate_suitability(biome.climate, climate),
            ));
        }
    }

    if weighted.is_empty() {
        return None;
    }
    if let Some(index) = pick_weighted(&weighted, hash.rotate_left(17)) {
        return Some(index);
    }

    for (index, weight) in &mut weighted {
        *weight = biomes[*index].weight;
    }
    pick_weighted(&weighted, hash.rotate_left(29))
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

    #[test]
    fn dominant_neighbor_detection_requires_a_real_majority() {
        assert_eq!(dominant_neighbor_biome(&[1, 1, 1, 1, 1, 2, 3, 4]), Some(1));
        assert_eq!(dominant_neighbor_biome(&[1, 1, 1, 1, 2, 2, 3, 4]), None);
    }

    #[test]
    fn weighted_fallback_keeps_input_order_after_reusing_climate_storage() {
        let mut weighted = vec![(4, 0.0), (2, 0.0), (7, 0.0)];
        assert_eq!(pick_weighted(&weighted, 42), None);
        for (index, weight) in &mut weighted {
            *weight = match *index {
                4 => 1.0,
                2 => 2.0,
                7 => 3.0,
                _ => unreachable!(),
            };
        }
        assert_eq!(weighted, vec![(4, 1.0), (2, 2.0), (7, 3.0)]);
        assert!(matches!(pick_weighted(&weighted, 42), Some(4 | 2 | 7)));
    }
}
