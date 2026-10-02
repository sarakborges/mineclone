pub(super) mod fitting;

use bevy::prelude::*;

use crate::{
    content::biome::{BiomeClimate, BiomeClimateRange, BiomeVerticalRange},
    world::{
        deterministic::mix_hash_u64,
        macro_climate::{MacroClimateField, MacroClimateSample},
    },
};

use super::{
    BiomeField, BiomeFieldEntry,
    constants::{CLIMATE_BLEND_MARGIN, SITE_SEARCH_RADIUS},
    distribution::distribution_strength,
    spatial::{cell_hash, hash_unit, surface_site_position},
};

impl BiomeField {
    pub(super) fn select_surface_biome_index(&self, cell: IVec2, site: Vec2) -> usize {
        let climate = self.climate.sample(site);
        let cell_hash = cell_hash(cell, self.seed);

        // Ocean's authored continentalness core is a macro mask, not merely
        // another weighted land candidate. Without this guard, land biomes
        // can fragment the ocean core into isolated Voronoi cells.
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
                return ocean_index;
            }
        }

        let weighted_candidates = self.surface_weighted_candidates(cell, site, climate, cell_hash);
        let raw_index = weighted_candidates
            .first()
            .map(|candidate| candidate.index)
            .unwrap_or_else(|| {
                panic!(
                    "surface biome site {cell:?} has no biome compatible with climate/distribution constraints"
                )
            });

        let selection_context = SurfaceSelectionContext {
            cell,
            site,
            spacing: self.surface_site_spacing,
            seed: self.seed,
            biomes: &self.surface_biomes,
            spawn_oceans: self.spawn_oceans,
            ocean_surface_index: self.ocean_surface_index,
            climate_field: &self.climate,
        };

        if surface_constraints_allow(raw_index, &selection_context) {
            return raw_index;
        }

        if let Some(candidate) = weighted_candidates
            .iter()
            .find(|candidate| surface_constraints_allow(candidate.index, &selection_context))
        {
            return candidate.index;
        }

        panic!(
            "surface biome site {cell:?} has no biome compatible with fitted size and authored adjacency constraints"
        );
    }

    fn surface_weighted_candidates(
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
struct WeightedSurfaceCandidate {
    index: usize,
    weight: f32,
}

pub(super) struct SurfaceSelectionContext<'a> {
    pub(super) cell: IVec2,
    pub(super) site: Vec2,
    pub(super) spacing: Vec2,
    pub(super) seed: u64,
    pub(super) biomes: &'a [BiomeFieldEntry],
    pub(super) spawn_oceans: bool,
    pub(super) ocean_surface_index: Option<usize>,
    pub(super) climate_field: &'a MacroClimateField,
}

fn surface_constraints_allow(
    candidate_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    // The site lattice is built from the largest authored minimum radius and
    // its jitter preserves axis spacing. Minimum size is therefore a geometry
    // invariant, not a reason to eliminate a biome candidate. Eliminating it
    // here was what allowed min/max + adjacency to empty a site's domain.
    debug_assert!(
        surface_minimum_size_allows(candidate_index, context),
        "surface biome site {:?} violates the authored minimum-size lattice invariant",
        context.cell,
    );

    authored_adjacency_allows(candidate_index, context)
        && fitting::surface_size_allows(candidate_index, context)
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

fn authored_adjacency_allows(
    candidate_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    let candidate = &context.biomes[candidate_index];
    let raw_index = raw_surface_biome_index(
        context.cell,
        context.site,
        context.climate_field.sample(context.site),
        cell_hash(context.cell, context.seed),
        context.biomes,
        context.seed,
        context.spawn_oceans,
    );

    for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
        for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            let offset = IVec2::new(x, z);
            if offset == IVec2::ZERO {
                continue;
            }

            let neighbor_cell = context.cell + offset;
            let neighbor_site =
                surface_site_position(neighbor_cell, context.spacing, context.seed);
            if !surface_sites_share_border(
                context.cell,
                context.site,
                neighbor_cell,
                neighbor_site,
                context.spacing,
                context.seed,
            ) {
                continue;
            }

            let neighbor_index = raw_surface_biome_index(
                neighbor_cell,
                neighbor_site,
                context.climate_field.sample(neighbor_site),
                cell_hash(neighbor_cell, context.seed),
                context.biomes,
                context.seed,
                context.spawn_oceans,
            );
            let neighbor = &context.biomes[neighbor_index];
            let authored_conflict = authored_pair_conflicts(candidate, neighbor);
            let fitted_size_conflict = candidate.id != neighbor.id
                && fitting::boundary_fit_interval(candidate, neighbor, context.site, neighbor_site)
                    .is_none();

            if !authored_conflict && !fitted_size_conflict {
                continue;
            }

            // An alternate candidate is only a fallback after this site's raw
            // choice yielded. It may not displace an unrelated raw neighbor in
            // order to make itself legal.
            if candidate_index != raw_index {
                return false;
            }

            // Raw/raw conflicts have one deterministic winner. This applies to
            // both authored adjacency and a min/max pair that physically cannot
            // share their current edge. The losing site tries its next weighted
            // candidate; no constraint is relaxed.
            if !fitting::raw_conflict_left_wins(
                context.cell,
                context.site,
                candidate_index,
                neighbor_cell,
                neighbor_site,
                neighbor_index,
                context,
            ) {
                return false;
            }
        }
    }

    if candidate.require_near.is_empty() {
        return true;
    }

    for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
        for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            let offset = IVec2::new(x, z);
            if offset == IVec2::ZERO {
                continue;
            }

            let neighbor_cell = context.cell + offset;
            let neighbor_site =
                surface_site_position(neighbor_cell, context.spacing, context.seed);
            if !surface_sites_share_border(
                context.cell,
                context.site,
                neighbor_cell,
                neighbor_site,
                context.spacing,
                context.seed,
            ) {
                continue;
            }

            let neighbor_index = raw_surface_biome_index(
                neighbor_cell,
                neighbor_site,
                context.climate_field.sample(neighbor_site),
                cell_hash(neighbor_cell, context.seed),
                context.biomes,
                context.seed,
                context.spawn_oceans,
            );
            if candidate
                .require_near
                .iter()
                .any(|id| id == &context.biomes[neighbor_index].id)
            {
                return true;
            }
        }
    }

    false
}

fn authored_pair_conflicts(left: &BiomeFieldEntry, right: &BiomeFieldEntry) -> bool {
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

fn surface_minimum_size_allows(
    candidate_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    let candidate = &context.biomes[candidate_index];
    let minimum_radii = Vec2::new(candidate.size.x.min, candidate.size.z.min);

    for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
        for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            let offset = IVec2::new(x, z);
            if offset == IVec2::ZERO {
                continue;
            }

            let neighbor_cell = context.cell + offset;
            let neighbor_site =
                surface_site_position(neighbor_cell, context.spacing, context.seed);
            if !surface_sites_share_border(
                context.cell,
                context.site,
                neighbor_cell,
                neighbor_site,
                context.spacing,
                context.seed,
            ) {
                continue;
            }

            let half_delta = (neighbor_site - context.site) * 0.5;
            let normalized = Vec2::new(
                half_delta.x / minimum_radii.x,
                half_delta.y / minimum_radii.y,
            )
            .length();

            if normalized + f32::EPSILON < 1.0 {
                return false;
            }
        }
    }

    true
}

pub(super) fn region_claim_hash(cell: IVec2, biome_index: usize, seed: u64) -> u64 {
    let hash = cell_hash(cell, seed)
        ^ (biome_index as u64).wrapping_mul(0x517c_c1b7_2722_0a95)
        ^ 0x94d0_49bb_1331_11eb;
    mix_hash_u64(hash)
}

pub(super) fn raw_surface_biome_index(
    _cell: IVec2,
    site: Vec2,
    climate: MacroClimateSample,
    source_hash: u64,
    biomes: &[BiomeFieldEntry],
    seed: u64,
    spawn_oceans: bool,
) -> usize {
    let mut candidates = biomes
        .iter()
        .enumerate()
        .filter(|(_, biome)| biome.weight > 0.0)
        .filter(|(index, _)| spawn_oceans || *index != biomes.len().saturating_sub(1))
        .filter_map(|(index, biome)| {
            let distribution = biome
                .distributions
                .iter()
                .copied()
                .map(|distribution| {
                    distribution_strength(distribution, site, seed, biome.id.as_str())
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
        let left_hash = candidate_hash(source_hash, &biomes[left.index].id);
        let right_hash = candidate_hash(source_hash, &biomes[right.index].id);
        let left_score = -hash_unit(left_hash).ln() / left.weight.max(f32::MIN_POSITIVE);
        let right_score = -hash_unit(right_hash).ln() / right.weight.max(f32::MIN_POSITIVE);
        left_score
            .total_cmp(&right_score)
            .then_with(|| left.index.cmp(&right.index))
    });

    candidates
        .first()
        .map(|candidate| candidate.index)
        .unwrap_or(0)
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

    fn test_surface_entry(id: &str, group: Option<&str>) -> BiomeFieldEntry {
        BiomeFieldEntry {
            id: id.to_owned(),
            tags: Vec::new(),
            surface_constraints: None,
            distributions: vec![crate::content::biome_distribution::BiomeDistribution::Regional],
            size: crate::content::dimension::DimensionBiomeSize {
                x: crate::content::dimension::DimensionBiomeSizeAxis { min: 1.0, max: 1.0 },
                z: crate::content::dimension::DimensionBiomeSizeAxis { min: 1.0, max: 1.0 },
                y: None,
            },
            weight: 1.0,
            climate: BiomeClimate::default(),
            vertical_range: None,
            priority: 0,
            terrain: None,
            terrain_modifiers: Vec::new(),
            density_modifier: None,
            solid_block: None,
            density_seed: 0,
            avoid_near: Vec::new(),
            require_near: Vec::new(),
            exclusive_neighbor_group: group.map(str::to_owned),
            surface_margin: None,
        }
    }

    fn test_context<'a>(
        cell: IVec2,
        spacing: Vec2,
        biomes: &'a [BiomeFieldEntry],
        climate: &'a MacroClimateField,
    ) -> SurfaceSelectionContext<'a> {
        SurfaceSelectionContext {
            cell,
            site: surface_site_position(cell, spacing, 42),
            spacing,
            seed: 42,
            biomes,
            spawn_oceans: true,
            ocean_surface_index: None,
            climate_field: climate,
        }
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

    #[test]
    fn exclusive_group_allows_same_biome_continuity() {
        let mountain = test_surface_entry("mountain", Some("mountain_terrain"));
        let climate = MacroClimateField::new(42);
        let biomes = [mountain];
        let context = test_context(IVec2::ZERO, Vec2::splat(360.0), &biomes, &climate);

        assert!(authored_adjacency_allows(0, &context));
    }

    #[test]
    fn minimum_size_guard_detects_an_artificially_compressed_lattice() {
        let mut plains = test_surface_entry("plains", None);
        plains.size.x = crate::content::dimension::DimensionBiomeSizeAxis {
            min: 180.0,
            max: 420.0,
        };
        plains.size.z = crate::content::dimension::DimensionBiomeSizeAxis {
            min: 180.0,
            max: 420.0,
        };
        let climate = MacroClimateField::new(42);
        let biomes = [plains];
        let context = test_context(IVec2::ZERO, Vec2::splat(300.0), &biomes, &climate);

        assert!(!surface_minimum_size_allows(0, &context));
    }

    #[test]
    fn minimum_size_guard_accepts_authored_spacing() {
        let mut plains = test_surface_entry("plains", None);
        plains.size.x = crate::content::dimension::DimensionBiomeSizeAxis {
            min: 180.0,
            max: 420.0,
        };
        plains.size.z = crate::content::dimension::DimensionBiomeSizeAxis {
            min: 180.0,
            max: 420.0,
        };
        let climate = MacroClimateField::new(42);
        let biomes = [plains];
        let context = test_context(IVec2::ZERO, Vec2::splat(360.0), &biomes, &climate);

        assert!(surface_minimum_size_allows(0, &context));
    }

    #[test]
    fn raw_exclusive_conflict_has_exactly_one_winner() {
        let left = test_surface_entry("left", Some("inland"));
        let right = test_surface_entry("right", Some("inland"));
        let fallback = test_surface_entry("fallback", None);
        let biomes = [left, right, fallback];
        let climate = MacroClimateField::new(42);
        let spacing = Vec2::splat(360.0);
        let left_cell = IVec2::ZERO;
        let right_cell = IVec2::X;
        let left_site = surface_site_position(left_cell, spacing, 42);
        let right_site = surface_site_position(right_cell, spacing, 42);
        let left_context = test_context(left_cell, spacing, &biomes, &climate);
        let right_context = test_context(right_cell, spacing, &biomes, &climate);

        let left_wins = fitting::raw_conflict_left_wins(
            left_cell,
            left_site,
            0,
            right_cell,
            right_site,
            1,
            &left_context,
        );
        let right_wins = fitting::raw_conflict_left_wins(
            right_cell,
            right_site,
            1,
            left_cell,
            left_site,
            0,
            &right_context,
        );

        assert_ne!(left_wins, right_wins);
    }
}
