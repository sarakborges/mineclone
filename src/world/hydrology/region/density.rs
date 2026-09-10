use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{OCEAN_EXTRA_DEPTH, OCEAN_MINIMUM_DEPTH, RIVER_CARVE_STRENGTH},
    math::smoothstep,
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
        self.water_bodies
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

                -(body.carve_depth + 2.0) * strength
            })
            .sum()
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
