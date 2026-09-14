use bevy::prelude::*;

use super::{
    BiomeField, BiomeFieldSample, BiomeInfluence,
    constants::{BORDER_TRANSITION_WIDTH, SITE_SEARCH_RADIUS},
    mountain_belt::mountain_belt_strength,
    mountain_peak::mountain_peak_strength,
    selection::select_surface_biome_index,
    spatial::{smoothstep, surface_site_position, warp_surface_position},
};

impl BiomeField {
    pub fn sample_surface(&self, position: Vec2) -> BiomeFieldSample<'_> {
        let warped = warp_surface_position(position, self.seed);
        let center = IVec2::new(
            (warped.x / self.surface_site_spacing.x).round() as i32,
            (warped.y / self.surface_site_spacing.y).round() as i32,
        );
        let mut sampled_sites = Vec::new();

        {
            let cache = self
                .surface_site_biomes
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                    let cell = center + IVec2::new(x, z);
                    let site = surface_site_position(cell, self.surface_site_spacing, self.seed);
                    sampled_sites.push((cell, site, warped.distance(site), cache.get(&cell).copied()));
                }
            }
        }

        let mut cache_updates = Vec::new();
        for (cell, site, _, candidate_index) in &mut sampled_sites {
            if candidate_index.is_some() {
                continue;
            }

            let selected = select_surface_biome_index(
                *cell,
                *site,
                self.surface_site_spacing,
                &self.surface_biomes,
                &self.climate,
                self.seed,
                self.ocean_biome_id.as_deref(),
                self.ocean_weight,
            );
            *candidate_index = Some(selected);
            cache_updates.push((*cell, selected));
        }

        if !cache_updates.is_empty() {
            let mut cache = self
                .surface_site_biomes
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for (cell, selected) in cache_updates {
                cache.entry(cell).or_insert(selected);
            }
        }

        let mut nearest_distance = f32::MAX;
        let mut primary_index = 0;
        let mut sites = Vec::with_capacity(sampled_sites.len());
        for (_, _, distance, candidate_index) in sampled_sites {
            let candidate_index = candidate_index.expect("surface biome site must be resolved");
            if distance < nearest_distance {
                nearest_distance = distance;
                primary_index = candidate_index;
            }
            sites.push((candidate_index, distance));
        }

        let mut weights = vec![0.0_f32; self.surface_biomes.len()];
        for (candidate_index, distance) in sites {
            let distance_gap = (distance - nearest_distance).max(0.0);
            let border_progress = 1.0 - (distance_gap / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
            let smooth_progress = smoothstep(border_progress);
            weights[candidate_index] = weights[candidate_index].max(smooth_progress);
        }

        let regional_total: f32 = weights.iter().sum();
        if regional_total > f32::EPSILON {
            for weight in &mut weights {
                *weight /= regional_total;
            }
        }

        let strongest_macro = self
            .surface_biomes
            .iter()
            .enumerate()
            .filter_map(|(index, biome)| {
                let strength = biome
                    .distributions
                    .iter()
                    .copied()
                    .map(|distribution| {
                        mountain_belt_strength(
                            distribution,
                            position,
                            self.seed,
                            biome.id.as_str(),
                        )
                        .max(mountain_peak_strength(
                            distribution,
                            position,
                            self.seed,
                            biome.id.as_str(),
                        ))
                    })
                    .fold(0.0_f32, f32::max)
                    * biome.weight;
                let strength = strength.clamp(0.0, 1.0);

                (strength > 0.0).then_some((index, strength))
            })
            .max_by(|left, right| left.1.total_cmp(&right.1));

        if let Some((macro_index, macro_strength)) = strongest_macro {
            let retained_regional_weight = 1.0 - macro_strength;
            for weight in &mut weights {
                *weight *= retained_regional_weight;
            }
            weights[macro_index] += macro_strength;
        }

        if let Some((index, _)) = weights
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
        {
            primary_index = index;
        }

        let total_weight: f32 = weights.iter().sum();
        let influences = weights
            .into_iter()
            .enumerate()
            .filter_map(|(index, weight)| {
                (weight > 0.0).then_some(BiomeInfluence {
                    id: self.surface_biomes[index].id.as_str(),
                    weight: weight / total_weight,
                })
            })
            .collect();

        BiomeFieldSample {
            primary_id: self.surface_biomes[primary_index].id.as_str(),
            influences,
        }
    }
}
