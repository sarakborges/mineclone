use arrayvec::ArrayVec;
use bevy::prelude::*;

use super::{
    BiomeField, BiomeFieldSample, BiomeInfluence, MAX_SURFACE_INFLUENCES, SurfaceBoundarySample,
    constants::{BORDER_TRANSITION_WIDTH, SITE_SEARCH_RADIUS},
    spatial::{smoothstep, varied_surface_margin_width, warp_surface_position},
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

        let warped = warp_surface_position(position, self.seed);
        let center = IVec2::new(
            (warped.x / self.surface_site_spacing.x).round() as i32,
            (warped.y / self.surface_site_spacing.y).round() as i32,
        );
        self.surface_map.ensure_sample_window(self, center);

        let mut sampled_sites = [(Vec2::ZERO, 0.0_f32, 0_usize); SITE_SAMPLE_COUNT];
        let mut sample_count = 0;
        {
            let samples = self
                .surface_map
                .read_samples()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                    let cell = center + IVec2::new(x, z);
                    let sample = samples.get(&cell).unwrap_or_else(|| {
                        panic!("surface biome map sample missing resolved cell {cell:?}")
                    });
                    sampled_sites[sample_count] = (
                        sample.position,
                        warped.distance(sample.position),
                        sample.biome_index,
                    );
                    sample_count += 1;
                }
            }
        }
        debug_assert_eq!(sample_count, SITE_SAMPLE_COUNT);

        let mut nearest_score = f32::INFINITY;
        let mut nearest_site = Vec2::ZERO;
        let mut nearest_sample_index = 0;
        let mut primary_index = 0;
        for (sample_index, (site, distance, candidate_index)) in
            sampled_sites[..sample_count].iter().enumerate()
        {
            let score = distance * distance;
            if score < nearest_score {
                nearest_score = score;
                nearest_site = *site;
                nearest_sample_index = sample_index;
                primary_index = *candidate_index;
            }
        }
        let regional_boundary = nearest_surface_boundary(
            nearest_sample_index,
            nearest_score,
            &sampled_sites[..sample_count],
        );

        let mut weights = [(usize::MAX, 0.0_f32); MAX_WEIGHT_ENTRIES];
        let mut weight_count = 0;
        for (site, distance, candidate_index) in &sampled_sites[..sample_count] {
            let pair_distance = nearest_site.distance(*site);
            let boundary_distance = if pair_distance <= f32::EPSILON {
                0.0
            } else {
                let candidate_score = distance * distance;
                ((candidate_score - nearest_score) / (2.0 * pair_distance)).max(0.0)
            };
            let border_progress =
                1.0 - (boundary_distance / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
            set_max_weight(
                &mut weights,
                &mut weight_count,
                *candidate_index,
                smoothstep(border_progress),
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
        let influences = weights[..weight_count]
            .iter()
            .filter(|(_, weight)| *weight > 0.0)
            .map(|(index, weight)| {
                let biome = &self.surface_biomes[*index];
                BiomeInfluence {
                    id: biome.id.as_str(),
                    weight: *weight / total_weight,
                    surface_index: *index,
                    terrain_strength: 1.0,
                }
            })
            .collect::<ArrayVec<_, MAX_SURFACE_INFLUENCES>>();

        let surface_margin_index = regional_boundary.and_then(|boundary| {
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

fn nearest_surface_boundary(
    primary_sample_index: usize,
    primary_score: f32,
    sampled_sites: &[(Vec2, f32, usize)],
) -> Option<SurfaceBoundarySample> {
    let (primary_site, _, primary_index) = sampled_sites[primary_sample_index];

    sampled_sites
        .iter()
        .filter_map(|(site, distance, candidate_index)| {
            if *candidate_index == primary_index {
                return None;
            }

            let pair_distance = primary_site.distance(*site);
            if pair_distance <= f32::EPSILON {
                return None;
            }

            let candidate_score = distance * distance;
            let boundary_distance =
                ((candidate_score - primary_score) / (2.0 * pair_distance)).max(0.0);
            Some(SurfaceBoundarySample {
                neighbor_surface_index: *candidate_index,
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
