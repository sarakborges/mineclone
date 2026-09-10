use bevy::prelude::*;

use super::{
    constants::{BORDER_TRANSITION_WIDTH, SITE_SEARCH_RADIUS},
    selection::select_surface_biome_index,
    spatial::{smoothstep, surface_site_position, warp_surface_position},
    BiomeField, BiomeFieldSample, BiomeInfluence,
};

impl BiomeField {
    pub fn sample_surface(&self, position: Vec2) -> BiomeFieldSample<'_> {
        if self.surface_biomes.len() == 1 {
            let id = self.surface_biomes[0].id.as_str();
            return BiomeFieldSample {
                primary_id: id,
                influences: vec![BiomeInfluence { id, weight: 1.0 }],
            };
        }

        let warped = warp_surface_position(position, self.seed);
        let center = IVec2::new(
            (warped.x / self.surface_site_spacing.x).round() as i32,
            (warped.y / self.surface_site_spacing.y).round() as i32,
        );
        let mut sites = Vec::new();
        let mut nearest_distance = f32::MAX;
        let mut primary_index = 0;

        for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                let cell = center + IVec2::new(x, z);
                let site = surface_site_position(cell, self.surface_site_spacing, self.seed);
                let distance = warped.distance(site);
                let candidate_index = select_surface_biome_index(
                    cell,
                    site,
                    &self.surface_biomes,
                    &self.climate,
                    self.seed,
                );

                if distance < nearest_distance {
                    nearest_distance = distance;
                    primary_index = candidate_index;
                }

                sites.push((candidate_index, distance));
            }
        }

        let mut weights = vec![0.0_f32; self.surface_biomes.len()];
        for (candidate_index, distance) in sites {
            let distance_gap = (distance - nearest_distance).max(0.0);
            let border_progress =
                1.0 - (distance_gap / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
            let smooth_progress = smoothstep(border_progress);
            weights[candidate_index] = weights[candidate_index].max(smooth_progress);
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
