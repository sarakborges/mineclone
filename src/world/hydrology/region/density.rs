use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{
        LAKE_SHORE_INNER_DISTANCE, LAKE_SHORE_OUTER_DISTANCE, LAKE_SHORE_SURFACE_OFFSET,
        OCEAN_EXTRA_DEPTH, OCEAN_MINIMUM_DEPTH, RIVER_CARVE_STRENGTH,
    },
    math::smoothstep,
    types::WaterBody,
};

impl HydrologyRegion {
    pub fn density_delta(&self, position: Vec3) -> f32 {
        let horizontal = Vec2::new(position.x, position.z);

        self.ocean_density_delta(horizontal)
            + self.river_density_delta(position, horizontal)
            + self.water_body_density_delta(position, horizontal)
    }

    fn river_density_delta(&self, position: Vec3, horizontal: Vec2) -> f32 {
        self.river_graph
            .sample_horizontal(horizontal)
            .map_or(0.0, |sample| {
                let profile = smoothstep(sample.strength);
                let bed = sample.height - self.river_carve_depth * profile;

                if position.y < bed - 0.5 || position.y > sample.height + 1.5 {
                    return 0.0;
                }

                -RIVER_CARVE_STRENGTH * profile
            })
    }

    fn water_body_density_delta(&self, position: Vec3, horizontal: Vec2) -> f32 {
        let carve_delta = self
            .water_bodies
            .iter()
            .map(|body| {
                if body.carve_depth <= 0.0 {
                    return 0.0;
                }

                let strength = body.horizontal_strength(horizontal);
                let bottom = body.water_level - body.carve_depth * strength;

                if strength <= 0.0
                    || position.y < bottom - 0.5
                    || position.y > body.water_level + 0.5
                {
                    return 0.0;
                }

                -(body.carve_depth * 1.8 + 2.0) * strength
            })
            .sum::<f32>();
        let shore_delta = self
            .water_bodies
            .iter()
            .map(|body| self.lake_shore_density_delta(body, horizontal))
            .max_by(f32::total_cmp)
            .unwrap_or(0.0);

        carve_delta + shore_delta
    }

    fn lake_shore_density_delta(&self, body: &WaterBody, horizontal: Vec2) -> f32 {
        let shore_strength = lake_shore_strength(body.normalized_horizontal_distance(horizontal));
        if shore_strength <= 0.0 {
            return 0.0;
        }

        let river_opening = self
            .river_graph
            .sample_horizontal(horizontal)
            .map_or(0.0, |river| smoothstep((river.strength * 2.0).clamp(0.0, 1.0)));
        if river_opening >= 1.0 {
            return 0.0;
        }

        let Some(surface) = self.macro_sample_at(horizontal) else {
            return 0.0;
        };
        let target_surface = body.water_level + LAKE_SHORE_SURFACE_OFFSET;
        let missing_height = (target_surface - surface.elevation).max(0.0);

        missing_height * shore_strength * (1.0 - river_opening)
    }

    fn ocean_density_delta(&self, position: Vec2) -> f32 {
        let Some(sample) = self.macro_sample_at(position) else {
            return 0.0;
        };
        let strength = self.ocean_strength_at(position);

        if strength <= 0.0 {
            return 0.0;
        }

        let target_floor = self.sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * strength;
        (target_floor - sample.elevation).min(0.0) * strength
    }
}

fn lake_shore_strength(distance: f32) -> f32 {
    if distance < LAKE_SHORE_INNER_DISTANCE || distance > LAKE_SHORE_OUTER_DISTANCE {
        return 0.0;
    }

    let progress = if distance <= 1.0 {
        (distance - LAKE_SHORE_INNER_DISTANCE) / (1.0 - LAKE_SHORE_INNER_DISTANCE)
    } else {
        1.0 - (distance - 1.0) / (LAKE_SHORE_OUTER_DISTANCE - 1.0)
    };

    smoothstep(progress.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lake_shore_peaks_at_water_boundary_and_fades_on_both_sides() {
        assert_eq!(lake_shore_strength(0.5), 0.0);
        assert!(lake_shore_strength(0.9) > 0.0);
        assert_eq!(lake_shore_strength(1.0), 1.0);
        assert!(lake_shore_strength(1.1) > 0.0);
        assert_eq!(lake_shore_strength(1.3), 0.0);
    }
}
