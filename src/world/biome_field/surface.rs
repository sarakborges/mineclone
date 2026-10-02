use arrayvec::ArrayVec;
use bevy::prelude::*;

use crate::content::biome_distribution::BiomeDistribution;

use super::{
    BiomeField, BiomeFieldSample, BiomeInfluence, MAX_SURFACE_INFLUENCES,
    SurfaceBoundarySample, SurfaceSiteCacheEntry,
    constants::{BORDER_TRANSITION_WIDTH, SITE_SEARCH_RADIUS},
    distribution::distribution_strength,
    selection::{fitting::fit_surface_site_weights, resolver::resolve_surface_sites},
    spatial::{
        smoothstep, surface_site_position, varied_surface_margin_width,
        warp_surface_position,
    },
};

const SITE_SEARCH_DIAMETER: usize = (SITE_SEARCH_RADIUS * 2 + 1) as usize;
const SITE_SAMPLE_COUNT: usize = SITE_SEARCH_DIAMETER * SITE_SEARCH_DIAMETER;
const MAX_WEIGHT_ENTRIES: usize = MAX_SURFACE_INFLUENCES;

impl BiomeField {
    pub fn sample_surface(&self, position: Vec2) -> BiomeFieldSample<'_> {
        if let Some(index) = self.single_surface_biome {
            let biome = &self.surface_biomes[index];
            let mut influences = ArrayVec::new();
            influences.push(BiomeInfluence {
                id: biome.id.as_str(),
                weight: 1.0,
                surface_index: index,
                terrain_strength: 1.0,
            });
            return BiomeFieldSample {
                primary_id: biome.id.as_str(),
                primary_surface_index: index,
                surface_margin_index: None,
                identity_surface_index: index,
                influences,
            };
        }

        // New worlds deliberately force the selected spawn biome around the
        // initial spawn point. Inside that authored core its weight is exactly
        // 1.0, so resolving the underlying recursive surface graph cannot affect
        // the returned identity or terrain contribution. Avoiding the solver
        // here keeps spawn probing from synchronously solving the same 5x5
        // constraint windows hundreds of times before loading can even begin.
        if let Some((index, weight)) = self.forced_surface_biome_at(position)
            && weight >= 1.0 - f32::EPSILON
        {
            let biome = &self.surface_biomes[index];
            let mut influences = ArrayVec::new();
            influences.push(BiomeInfluence {
                id: biome.id.as_str(),
                weight: 1.0,
                surface_index: index,
                terrain_strength: self.forced_surface_terrain_strength(index, position),
            });
            return BiomeFieldSample {
                primary_id: biome.id.as_str(),
                primary_surface_index: index,
                surface_margin_index: None,
                identity_surface_index: index,
                influences,
            };
        }

        let warped = warp_surface_position(position, self.seed);
        let center = IVec2::new(
            (warped.x / self.surface_site_spacing.x).round() as i32,
            (warped.y / self.surface_site_spacing.y).round() as i32,
        );
        let mut sampled_sites =
            [(IVec2::ZERO, Vec2::ZERO, 0.0_f32, None); SITE_SAMPLE_COUNT];
        let mut sample_count = 0;

        {
            let cache = self
                .surface_site_cache
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                    let cell = center + IVec2::new(x, z);
                    let cached = cache.get(&cell).copied();
                    let site = cached.map_or_else(
                        || surface_site_position(cell, self.surface_site_spacing, self.seed),
                        |entry| entry.position,
                    );
                    sampled_sites[sample_count] = (
                        cell,
                        site,
                        warped.distance(site),
                        cached.map(|entry| entry.biome_index),
                    );
                    sample_count += 1;
                }
            }
        }
        debug_assert_eq!(sample_count, SITE_SAMPLE_COUNT);

        // Resolve the canonical window as one constraint problem before any new
        // identity is cached. The recursive solver is allowed to propagate past
        // this 5x5 window when a neighboring assignment must move in order for
        // min/max or authored adjacency to remain satisfiable.
        let needs_resolution = sampled_sites[..sample_count]
            .iter()
            .any(|(_, _, _, candidate_index)| candidate_index.is_none());
        if needs_resolution {
            let requested_cells = sampled_sites[..sample_count]
                .iter()
                .map(|(cell, _, _, _)| *cell)
                .collect::<ArrayVec<_, SITE_SAMPLE_COUNT>>();
            let resolved = resolve_surface_sites(self, requested_cells.as_slice())
                .unwrap_or_else(|reason| {
                    panic!("surface biome recursive fitting failed near site {center:?}: {reason}")
                });

            {
                let mut cache = self
                    .surface_site_cache
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                for &(cell, site, selected) in &resolved {
                    cache
                        .entry(cell)
                        .and_modify(|existing| {
                            debug_assert_eq!(
                                existing.biome_index, selected,
                                "recursive biome fitting must be independent of query order"
                            );
                        })
                        .or_insert(SurfaceSiteCacheEntry {
                            position: site,
                            biome_index: selected,
                        });
                }
            }

            for (cell, _, _, candidate_index) in &mut sampled_sites[..sample_count] {
                let selected = resolved
                    .iter()
                    .find_map(|(resolved_cell, _, selected)| {
                        (*resolved_cell == *cell).then_some(*selected)
                    })
                    .unwrap_or_else(|| {
                        panic!("recursive biome fitting omitted requested site {cell:?}")
                    });
                *candidate_index = Some(selected);
            }
        }

        // A center cell always resolves the same canonical 5x5 site window.
        // Cache its power-diagram solution so terrain sampling pays the graph
        // fitting cost once per site cell instead of once per world column.
        let site_weights = if let Some(weights) = self.surface_site_cache.fitted_weights(center) {
            weights
        } else {
            let weights = fit_surface_site_weights(
                &sampled_sites[..sample_count],
                &self.surface_biomes,
                self.surface_site_spacing,
                self.seed,
            )
            .unwrap_or_else(|reason| {
                panic!("surface biome boundary fitting failed near site {center:?}: {reason}")
            });
            debug_assert_eq!(weights.len(), sample_count);
            self.surface_site_cache
                .cache_fitted_weights(center, weights)
        };

        let mut nearest_score = f32::INFINITY;
        let mut nearest_site = Vec2::ZERO;
        let mut nearest_sample_index = 0;
        let mut primary_index = 0;
        for (sample_index, (_, site, distance, candidate_index)) in
            sampled_sites[..sample_count].iter().enumerate()
        {
            let candidate_index = candidate_index.expect("surface biome site must be resolved");
            let score = distance * distance - site_weights[sample_index];
            if score < nearest_score {
                nearest_score = score;
                nearest_site = *site;
                nearest_sample_index = sample_index;
                primary_index = candidate_index;
            }
        }
        let fitted_primary_index = primary_index;
        let fitted_boundary = nearest_surface_boundary(
            nearest_sample_index,
            nearest_score,
            &sampled_sites[..sample_count],
            site_weights.as_ref(),
        );

        let mut weights = [(usize::MAX, 0.0_f32); MAX_WEIGHT_ENTRIES];
        let mut weight_count = 0;
        for (sample_index, (_, site, distance, candidate_index)) in
            sampled_sites[..sample_count].iter().enumerate()
        {
            let candidate_index = candidate_index.expect("surface biome site must be resolved");
            let pair_distance = nearest_site.distance(*site);
            let boundary_distance = if pair_distance <= f32::EPSILON {
                0.0
            } else {
                let candidate_score = distance * distance - site_weights[sample_index];
                ((candidate_score - nearest_score) / (2.0 * pair_distance)).max(0.0)
            };
            let border_progress =
                1.0 - (boundary_distance / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
            let smooth_progress = smoothstep(border_progress);
            set_max_weight(
                &mut weights,
                &mut weight_count,
                candidate_index,
                smooth_progress,
            );
        }

        let regional_total: f32 = weights[..weight_count]
            .iter()
            .map(|(_, weight)| *weight)
            .sum();
        if regional_total > f32::EPSILON {
            for (_, weight) in &mut weights[..weight_count] {
                *weight /= regional_total;
            }
        }

        // Blend weights describe the transition across an already-fitted
        // boundary. They must never reassign topological ownership, otherwise
        // the blend can move the effective border back inside authored size.min.
        weights[..weight_count].sort_unstable_by_key(|(index, _)| *index);
        let total_weight: f32 = weights[..weight_count]
            .iter()
            .map(|(_, weight)| *weight)
            .sum();
        let mut influences = weights[..weight_count]
            .iter()
            .filter(|(_, weight)| *weight > 0.0)
            .map(|(index, weight)| {
                let biome = &self.surface_biomes[*index];
                let terrain_strength = biome
                    .distributions
                    .iter()
                    .copied()
                    .map(|distribution| {
                        distribution_strength(
                            distribution,
                            position,
                            self.seed,
                            biome.id.as_str(),
                        )
                    })
                    .fold(0.0_f32, f32::max)
                    .clamp(0.0, 1.0);

                BiomeInfluence {
                    id: self.surface_biomes[*index].id.as_str(),
                    weight: *weight / total_weight,
                    surface_index: *index,
                    terrain_strength,
                }
            })
            .collect::<ArrayVec<_, MAX_SURFACE_INFLUENCES>>();

        if let Some((forced_index, forced_weight)) = self.forced_surface_biome_at(position) {
            let forced_terrain_strength =
                self.forced_surface_terrain_strength(forced_index, position);
            for influence in &mut influences {
                influence.weight *= 1.0 - forced_weight;
            }
            if let Some(existing) = influences
                .iter_mut()
                .find(|influence| influence.surface_index == forced_index)
            {
                existing.weight += forced_weight;
                existing.terrain_strength =
                    existing.terrain_strength.max(forced_terrain_strength);
            } else {
                influences.push(BiomeInfluence {
                    id: self.surface_biomes[forced_index].id.as_str(),
                    weight: forced_weight,
                    surface_index: forced_index,
                    terrain_strength: forced_terrain_strength,
                });
            }

            primary_index = influences
                .iter()
                .filter(|influence| influence.weight > 0.0)
                .max_by(|left, right| {
                    left.weight
                        .total_cmp(&right.weight)
                        .then_with(|| left.surface_index.cmp(&right.surface_index))
                })
                .map(|influence| influence.surface_index)
                .unwrap_or(forced_index);
        }

        let nearest_boundary = (primary_index == fitted_primary_index)
            .then_some(fitted_boundary)
            .flatten();
        let surface_margin_index = nearest_boundary.and_then(|boundary| {
            let margin_owner = &self.surface_biomes[boundary.neighbor_surface_index];
            let margin = margin_owner.surface_margin?;
            let width = varied_surface_margin_width(
                position,
                margin.noise_seed,
                margin.width,
                margin.width_variation,
                margin.variation_scale,
            );
            (boundary.distance <= width).then_some(boundary.neighbor_surface_index)
        });
        let identity_surface_index = surface_margin_index.unwrap_or(primary_index);

        BiomeFieldSample {
            primary_id: self.surface_biomes[identity_surface_index].id.as_str(),
            primary_surface_index: primary_index,
            surface_margin_index,
            identity_surface_index,
            influences,
        }
    }
}

impl BiomeField {
    fn forced_surface_terrain_strength(&self, biome_index: usize, position: Vec2) -> f32 {
        let forced = self
            .forced_surface_biome
            .expect("forced terrain strength requires a forced surface biome");
        debug_assert_eq!(forced.biome_index, biome_index);

        let delta = forced.warped_delta(position);
        let normalized = Vec2::new(delta.x / forced.radii.x, delta.y / forced.radii.y);
        let radial_distance = normalized.length();
        let lateral_distance = if forced.radii.x <= forced.radii.y {
            normalized.x.abs()
        } else {
            normalized.y.abs()
        };

        self.surface_biomes[biome_index]
            .distributions
            .iter()
            .copied()
            .map(|distribution| {
                forced_distribution_strength(distribution, radial_distance, lateral_distance)
            })
            .fold(0.0_f32, f32::max)
            .clamp(0.0, 1.0)
    }
}

fn forced_distribution_strength(
    distribution: BiomeDistribution,
    radial_distance: f32,
    lateral_distance: f32,
) -> f32 {
    match distribution {
        BiomeDistribution::Regional => 1.0,
        BiomeDistribution::MountainPeak { .. } => {
            smoothstep((1.0 - radial_distance).clamp(0.0, 1.0))
        }
        BiomeDistribution::NoiseBand { .. } | BiomeDistribution::MountainBelt { .. } => {
            smoothstep((1.0 - lateral_distance).clamp(0.0, 1.0))
        }
    }
}

fn nearest_surface_boundary(
    primary_sample_index: usize,
    primary_score: f32,
    sampled_sites: &[(IVec2, Vec2, f32, Option<usize>)],
    site_weights: &[f32],
) -> Option<SurfaceBoundarySample> {
    let (_, primary_site, _, primary_index) = sampled_sites[primary_sample_index];
    let primary_index = primary_index.expect("surface biome site must be resolved");

    sampled_sites
        .iter()
        .enumerate()
        .filter_map(|(sample_index, (_, site, distance, candidate_index))| {
            let candidate_index = candidate_index.expect("surface biome site must be resolved");
            if candidate_index == primary_index {
                return None;
            }

            let pair_distance = primary_site.distance(*site);
            if pair_distance <= f32::EPSILON {
                return None;
            }

            let candidate_score = distance * distance - site_weights[sample_index];
            let boundary_distance =
                ((candidate_score - primary_score) / (2.0 * pair_distance)).max(0.0);
            Some(SurfaceBoundarySample {
                neighbor_surface_index: candidate_index,
                distance: boundary_distance,
            })
        })
        .min_by(|left, right| left.distance.total_cmp(&right.distance))
}

fn set_max_weight(
    weights: &mut [(usize, f32); MAX_WEIGHT_ENTRIES],
    weight_count: &mut usize,
    index: usize,
    weight: f32,
) {
    if let Some((_, existing)) = weights[..*weight_count]
        .iter_mut()
        .find(|(candidate, _)| *candidate == index)
    {
        *existing = existing.max(weight);
        return;
    }

    push_weight(weights, weight_count, index, weight);
}

fn push_weight(
    weights: &mut [(usize, f32); MAX_WEIGHT_ENTRIES],
    weight_count: &mut usize,
    index: usize,
    weight: f32,
) {
    debug_assert!(*weight_count < weights.len());
    weights[*weight_count] = (index, weight);
    *weight_count += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_weights_keep_maximum_per_biome() {
        let mut weights = [(usize::MAX, 0.0_f32); MAX_WEIGHT_ENTRIES];
        let mut count = 0;

        set_max_weight(&mut weights, &mut count, 4, 0.25);
        set_max_weight(&mut weights, &mut count, 4, 0.75);
        set_max_weight(&mut weights, &mut count, 2, 0.50);
        set_max_weight(&mut weights, &mut count, 7, 0.20);

        assert_eq!(count, 3);
        assert_eq!(weights[0], (4, 0.75));
        assert_eq!(weights[1], (2, 0.50));
        assert_eq!(weights[2], (7, 0.20));
    }

    #[test]
    fn compact_weight_ties_prefer_higher_biome_index_like_dense_iteration() {
        let weights = [(2, 0.5_f32), (7, 0.5_f32), (4, 0.25_f32)];
        let primary = weights
            .iter()
            .copied()
            .max_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.0.cmp(&right.0))
            })
            .map(|(index, _)| index);

        assert_eq!(primary, Some(7));
    }
}
