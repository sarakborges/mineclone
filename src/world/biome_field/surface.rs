use arrayvec::ArrayVec;
use bevy::prelude::*;

use super::{
    BiomeField, BiomeFieldSample, BiomeInfluence, MAX_SURFACE_INFLUENCES, SurfaceBoundarySample,
    constants::{BORDER_TRANSITION_WIDTH, SITE_SEARCH_RADIUS},
    spatial::{smoothstep, surface_site_position, varied_surface_margin_width},
};

const SITE_SEARCH_DIAMETER: usize = (SITE_SEARCH_RADIUS * 2 + 1) as usize;
const SITE_SAMPLE_COUNT: usize = SITE_SEARCH_DIAMETER * SITE_SEARCH_DIAMETER;
const MAX_WEIGHT_ENTRIES: usize = MAX_SURFACE_INFLUENCES;

impl BiomeField {
    pub fn sample_surface(&self, position: Vec2) -> BiomeFieldSample<'_> {
        if let Some(index) = self.single_surface_biome {
            return single_biome_sample(self, index);
        }

        let primary_index = self.surface_biome_index_at(position);
        let center = IVec2::new(
            (position.x / self.surface_site_spacing.x).round() as i32,
            (position.y / self.surface_site_spacing.y).round() as i32,
        );

        let mut sampled_sites = [(Vec2::ZERO, 0_usize); SITE_SAMPLE_COUNT];
        let mut sample_count = 0;
        for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                let cell = center + IVec2::new(x, z);
                let site = surface_site_position(cell, self.surface_site_spacing, self.seed);
                sampled_sites[sample_count] = (site, self.surface_biome_index_at(site));
                sample_count += 1;
            }
        }
        debug_assert_eq!(sample_count, SITE_SAMPLE_COUNT);

        let primary_site = sampled_sites[..sample_count]
            .iter()
            .filter(|(_, index)| *index == primary_index)
            .min_by(|left, right| {
                position
                    .distance_squared(left.0)
                    .total_cmp(&position.distance_squared(right.0))
            })
            .map(|(site, _)| *site)
            .unwrap_or(position);

        let regional_boundary = nearest_surface_boundary(
            position,
            primary_site,
            primary_index,
            &sampled_sites[..sample_count],
        );

        let mut weights = [(usize::MAX, 0.0_f32); MAX_WEIGHT_ENTRIES];
        let mut weight_count = 0;
        set_max_weight(&mut weights, &mut weight_count, primary_index, 1.0);

        for (site, candidate_index) in &sampled_sites[..sample_count] {
            if *candidate_index == primary_index {
                continue;
            }
            let boundary_distance = bisector_distance(position, primary_site, *site);
            if boundary_distance > BORDER_TRANSITION_WIDTH {
                continue;
            }
            let border_progress =
                1.0 - (boundary_distance / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
            set_max_weight(
                &mut weights,
                &mut weight_count,
                *candidate_index,
                smoothstep(border_progress),
            );
        }

        let total_weight = weights[..weight_count]
            .iter()
            .map(|(_, weight)| *weight)
            .sum::<f32>()
            .max(f32::MIN_POSITIVE);
        weights[..weight_count].sort_unstable_by_key(|(index, _)| *index);
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

fn single_biome_sample(field: &BiomeField, index: usize) -> BiomeFieldSample<'_> {
    let biome = &field.surface_biomes[index];
    let mut influences = ArrayVec::new();
    influences.push(BiomeInfluence {
        id: biome.id.as_str(),
        weight: 1.0,
        surface_index: index,
        terrain_strength: 1.0,
    });
    BiomeFieldSample {
        primary_id: biome.id.as_str(),
        primary_surface_index: index,
        surface_margin_index: None,
        identity_surface_index: index,
        influences,
    }
}

fn nearest_surface_boundary(
    position: Vec2,
    primary_site: Vec2,
    primary_index: usize,
    sampled_sites: &[(Vec2, usize)],
) -> Option<SurfaceBoundarySample> {
    sampled_sites
        .iter()
        .filter(|(_, candidate_index)| *candidate_index != primary_index)
        .map(|(site, candidate_index)| SurfaceBoundarySample {
            neighbor_surface_index: *candidate_index,
            distance: bisector_distance(position, primary_site, *site),
        })
        .min_by(|left, right| {
            left.distance
                .total_cmp(&right.distance)
                .then_with(|| left.neighbor_surface_index.cmp(&right.neighbor_surface_index))
        })
}

fn bisector_distance(position: Vec2, primary_site: Vec2, candidate_site: Vec2) -> f32 {
    let pair_distance = primary_site.distance(candidate_site);
    if pair_distance <= f32::EPSILON {
        return f32::INFINITY;
    }

    let primary_score = position.distance_squared(primary_site);
    let candidate_score = position.distance_squared(candidate_site);
    ((candidate_score - primary_score).abs() / (2.0 * pair_distance)).max(0.0)
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
    fn far_neighbor_has_no_transition_weight() {
        let distance = bisector_distance(Vec2::ZERO, Vec2::ZERO, Vec2::new(128.0, 0.0));
        assert_eq!(distance, 64.0);
        assert!(distance > BORDER_TRANSITION_WIDTH);
    }
}
