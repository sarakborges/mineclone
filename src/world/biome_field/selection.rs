use bevy::prelude::*;

use crate::{
    content::biome::{BiomeClimate, BiomeClimateRange, BiomeVerticalRange},
    world::macro_climate::MacroClimateSample,
};

use super::{
    BiomeField, BiomeFieldEntry,
    constants::{CLIMATE_BLEND_MARGIN, SITE_SEARCH_RADIUS},
    distribution::distribution_strength,
    spatial::{cell_hash, hash_unit, surface_site_position},
};

const PROXIMITY_SITE_RADIUS: i32 = 1;
const PROXIMITY_NEIGHBOR_COUNT: usize =
    ((PROXIMITY_SITE_RADIUS * 2 + 1) * (PROXIMITY_SITE_RADIUS * 2 + 1) - 1) as usize;

struct SurfaceAdjacencyContext<'a> {
    cell: IVec2,
    site: Vec2,
    nearby_cells: &'a [IVec2; PROXIMITY_NEIGHBOR_COUNT],
    nearby_sites: &'a [Vec2; PROXIMITY_NEIGHBOR_COUNT],
    nearby_biomes: &'a [usize; PROXIMITY_NEIGHBOR_COUNT],
    biomes: &'a [BiomeFieldEntry],
    spacing: Vec2,
    seed: u64,
}

impl BiomeField {
    pub(super) fn select_surface_biome_index(&self, cell: IVec2, site: Vec2) -> usize {
        // avoidNear means "these biome regions may not share a border", not
        // "there must be an exclusion radius". Keep the immediate neighboring
        // sites, then test which of them actually share a Voronoi edge with
        // this site before applying the symmetric conflict rule.
        let mut nearby_cells = [IVec2::ZERO; PROXIMITY_NEIGHBOR_COUNT];
        let mut nearby_sites = [Vec2::ZERO; PROXIMITY_NEIGHBOR_COUNT];
        let mut nearby_biomes = [0; PROXIMITY_NEIGHBOR_COUNT];
        let mut nearby_count = 0;

        for z in -PROXIMITY_SITE_RADIUS..=PROXIMITY_SITE_RADIUS {
            for x in -PROXIMITY_SITE_RADIUS..=PROXIMITY_SITE_RADIUS {
                if x == 0 && z == 0 {
                    continue;
                }

                let neighbor_cell = cell + IVec2::new(x, z);
                let neighbor_site =
                    surface_site_position(neighbor_cell, self.surface_site_spacing, self.seed);
                nearby_cells[nearby_count] = neighbor_cell;
                nearby_sites[nearby_count] = neighbor_site;
                nearby_biomes[nearby_count] =
                    self.raw_surface_biome_index(neighbor_cell, neighbor_site);
                nearby_count += 1;
            }
        }
        debug_assert_eq!(nearby_count, nearby_biomes.len());

        let climate = self.climate.sample(site);
        let hash = cell_hash(cell, self.seed);
        let adjacency = SurfaceAdjacencyContext {
            cell,
            site,
            nearby_cells: &nearby_cells,
            nearby_sites: &nearby_sites,
            nearby_biomes: &nearby_biomes,
            biomes: &self.surface_biomes,
            spacing: self.surface_site_spacing,
            seed: self.seed,
        };
        let raw_index = self.raw_surface_biome_index(cell, site);
        if adjacency_allows(&self.surface_biomes[raw_index], &adjacency) {
            return raw_index;
        }

        self.select_weighted_surface_biome_index(
            site,
            climate,
            hash.rotate_left(9),
            |candidate| adjacency_allows(candidate, &adjacency),
        )
        .unwrap_or_else(|| {
            panic!(
                "surface biome site {cell:?} has no biome compatible with avoidNear borders"
            )
        })
    }

    fn raw_surface_biome_index(&self, cell: IVec2, site: Vec2) -> usize {
        let climate = self.climate.sample(site);
        let hash = cell_hash(cell, self.seed);

        self.select_weighted_surface_biome_index(site, climate, hash, |_| true)
            .unwrap_or_else(|| {
                panic!("surface biome field has no active biome at site {cell:?}")
            })
    }

    fn select_weighted_surface_biome_index(
        &self,
        site: Vec2,
        climate: MacroClimateSample,
        hash: u64,
        predicate: impl Fn(&BiomeFieldEntry) -> bool,
    ) -> Option<usize> {
        let mut weighted = Vec::with_capacity(self.surface_biomes.len());
        for (index, biome) in self.surface_biomes.iter().enumerate() {
            if !predicate(biome) || biome.weight <= f32::EPSILON {
                continue;
            }

            let distribution = biome
                .distributions
                .iter()
                .copied()
                .map(|distribution| {
                    distribution_strength(distribution, site, self.seed, biome.id.as_str())
                })
                .fold(0.0_f32, f32::max)
                .clamp(0.0, 1.0);
            if distribution <= f32::EPSILON {
                continue;
            }

            weighted.push((
                index,
                biome.weight * distribution * climate_suitability(biome.climate, climate),
            ));
        }

        if weighted.is_empty() {
            return None;
        }
        if let Some(index) = pick_weighted(&weighted, hash.rotate_left(17)) {
            return Some(index);
        }

        for (index, weight) in &mut weighted {
            let biome = &self.surface_biomes[*index];
            let distribution = biome
                .distributions
                .iter()
                .copied()
                .map(|distribution| {
                    distribution_strength(distribution, site, self.seed, biome.id.as_str())
                })
                .fold(0.0_f32, f32::max)
                .clamp(0.0, 1.0);
            *weight = biome.weight * distribution;
        }
        pick_weighted(&weighted, hash.rotate_left(29))
    }
}

fn adjacency_allows(
    candidate: &BiomeFieldEntry,
    context: &SurfaceAdjacencyContext<'_>,
) -> bool {
    let mut required_neighbor_found = candidate.require_near.is_empty();

    for ((&neighbor_cell, &neighbor_site), &neighbor_index) in context
        .nearby_cells
        .iter()
        .zip(context.nearby_sites)
        .zip(context.nearby_biomes)
    {
        let neighbor = &context.biomes[neighbor_index];
        let shares_border = surface_sites_share_border(
            context.cell,
            context.site,
            neighbor_cell,
            neighbor_site,
            context.spacing,
            context.seed,
        );
        if !shares_border {
            continue;
        }

        if biomes_conflict(candidate, neighbor) {
            return false;
        }
        if candidate
            .require_near
            .iter()
            .any(|required| required == &neighbor.id)
        {
            required_neighbor_found = true;
        }
    }

    required_neighbor_found
}

fn surface_sites_share_border(
    left_cell: IVec2,
    left_site: Vec2,
    right_cell: IVec2,
    right_site: Vec2,
    spacing: Vec2,
    seed: u64,
) -> bool {
    let separation = right_site - left_site;
    let length = separation.length();
    if length <= f32::EPSILON {
        return false;
    }

    let midpoint = (left_site + right_site) * 0.5;
    let normal = Vec2::new(-separation.y, separation.x) / length;
    let left_from_midpoint = left_site - midpoint;
    let mut minimum_t = f32::NEG_INFINITY;
    let mut maximum_t = f32::INFINITY;

    let minimum_cell = left_cell.min(right_cell) - IVec2::splat(SITE_SEARCH_RADIUS);
    let maximum_cell = left_cell.max(right_cell) + IVec2::splat(SITE_SEARCH_RADIUS);

    for z in minimum_cell.y..=maximum_cell.y {
        for x in minimum_cell.x..=maximum_cell.x {
            let other_cell = IVec2::new(x, z);
            if other_cell == left_cell || other_cell == right_cell {
                continue;
            }

            let other_site = surface_site_position(other_cell, spacing, seed);
            let other_from_midpoint = other_site - midpoint;
            let coefficient =
                2.0 * normal.dot(other_from_midpoint - left_from_midpoint);
            let bound = other_from_midpoint.length_squared()
                - left_from_midpoint.length_squared();

            if coefficient > 1e-5 {
                maximum_t = maximum_t.min(bound / coefficient);
            } else if coefficient < -1e-5 {
                minimum_t = minimum_t.max(bound / coefficient);
            } else if bound < -1e-4 {
                return false;
            }

            if minimum_t > maximum_t + 1e-4 {
                return false;
            }
        }
    }

    minimum_t <= maximum_t + 1e-4
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
            weighted.push((index, biome.weight * climate_suitability(biome.climate, climate)));
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
    fn distant_sites_do_not_trigger_avoid_near() {
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
