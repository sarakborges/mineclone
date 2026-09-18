use arrayvec::ArrayVec;
use bevy::prelude::*;

use super::{
    BiomeField, BiomeFieldSample, BiomeInfluence, MAX_SURFACE_INFLUENCES,
    constants::{BORDER_TRANSITION_WIDTH, SITE_SEARCH_RADIUS},
    mountain_belt::mountain_belt_strength,
    mountain_peak::mountain_peak_strength,
    spatial::{smoothstep, surface_site_position, warp_surface_position},
};

const SITE_SEARCH_DIAMETER: usize = (SITE_SEARCH_RADIUS * 2 + 1) as usize;
const SITE_SAMPLE_COUNT: usize = SITE_SEARCH_DIAMETER * SITE_SEARCH_DIAMETER;
const MAX_WEIGHT_ENTRIES: usize = MAX_SURFACE_INFLUENCES;

impl BiomeField {
    pub fn sample_surface(&self, position: Vec2) -> BiomeFieldSample<'_> {
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
                .surface_site_biomes
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                    let cell = center + IVec2::new(x, z);
                    let site = surface_site_position(cell, self.surface_site_spacing, self.seed);
                    sampled_sites[sample_count] = (
                        cell,
                        site,
                        warped.distance(site),
                        cache.get(&cell).copied(),
                    );
                    sample_count += 1;
                }
            }
        }
        debug_assert_eq!(sample_count, SITE_SAMPLE_COUNT);

        let mut cache_updates = [None; SITE_SAMPLE_COUNT];
        let mut cache_update_count = 0;
        for (cell, site, _, candidate_index) in &mut sampled_sites[..sample_count] {
            if candidate_index.is_some() {
                continue;
            }

            let selected = self.select_surface_biome_index(*cell, *site);
            *candidate_index = Some(selected);
            cache_updates[cache_update_count] = Some((*cell, selected));
            cache_update_count += 1;
        }

        if cache_update_count > 0 {
            let mut cache = self
                .surface_site_biomes
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for update in &cache_updates[..cache_update_count] {
                let (cell, selected) =
                    update.expect("surface biome cache update must be initialized");
                cache.entry(cell).or_insert(selected);
            }
        }

        let mut nearest_distance = f32::MAX;
        let mut primary_index = 0;
        for (_, _, distance, candidate_index) in &sampled_sites[..sample_count] {
            let candidate_index = candidate_index.expect("surface biome site must be resolved");
            if *distance < nearest_distance {
                nearest_distance = *distance;
                primary_index = candidate_index;
            }
        }

        let mut weights = [(usize::MAX, 0.0_f32); MAX_WEIGHT_ENTRIES];
        let mut weight_count = 0;
        for (_, _, distance, candidate_index) in &sampled_sites[..sample_count] {
            let candidate_index = candidate_index.expect("surface biome site must be resolved");
            let distance_gap = (*distance - nearest_distance).max(0.0);
            let border_progress =
                1.0 - (distance_gap / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
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
                        mountain_belt_strength(distribution, position, self.seed, biome.id.as_str())
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
            for (_, weight) in &mut weights[..weight_count] {
                *weight *= retained_regional_weight;
            }
            add_weight(
                &mut weights,
                &mut weight_count,
                macro_index,
                macro_strength,
            );
        }

        if let Some((index, _)) = weights[..weight_count]
            .iter()
            .copied()
            .max_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.0.cmp(&right.0))
            })
        {
            primary_index = index;
        }

        weights[..weight_count].sort_unstable_by_key(|(index, _)| *index);
        let total_weight: f32 = weights[..weight_count]
            .iter()
            .map(|(_, weight)| *weight)
            .sum();
        let mut influences = weights[..weight_count]
            .iter()
            .filter(|(_, weight)| *weight > 0.0)
            .map(|(index, weight)| BiomeInfluence {
                id: self.surface_biomes[*index].id.as_str(),
                weight: *weight / total_weight,
                surface_index: *index,
            })
            .collect::<ArrayVec<_, MAX_SURFACE_INFLUENCES>>();

        if let Some((forced_index, forced_weight)) = self.forced_surface_biome_at(position) {
            for influence in &mut influences {
                influence.weight *= 1.0 - forced_weight;
            }
            if let Some(existing) = influences
                .iter_mut()
                .find(|influence| influence.surface_index == forced_index)
            {
                existing.weight += forced_weight;
            } else {
                influences.push(BiomeInfluence {
                    id: self.surface_biomes[forced_index].id.as_str(),
                    weight: forced_weight,
                    surface_index: forced_index,
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

        BiomeFieldSample {
            primary_id: self.surface_biomes[primary_index].id.as_str(),
            primary_surface_index: primary_index,
            influences,
        }
    }
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

fn add_weight(
    weights: &mut [(usize, f32); MAX_WEIGHT_ENTRIES],
    weight_count: &mut usize,
    index: usize,
    weight: f32,
) {
    if let Some((_, existing)) = weights[..*weight_count]
        .iter_mut()
        .find(|(candidate, _)| *candidate == index)
    {
        *existing += weight;
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
    fn compact_weights_keep_maximum_per_biome_and_add_macro_weight() {
        let mut weights = [(usize::MAX, 0.0_f32); MAX_WEIGHT_ENTRIES];
        let mut count = 0;

        set_max_weight(&mut weights, &mut count, 4, 0.25);
        set_max_weight(&mut weights, &mut count, 4, 0.75);
        set_max_weight(&mut weights, &mut count, 2, 0.50);
        add_weight(&mut weights, &mut count, 4, 0.10);
        add_weight(&mut weights, &mut count, 7, 0.20);

        assert_eq!(count, 3);
        assert_eq!(weights[0], (4, 0.85));
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
