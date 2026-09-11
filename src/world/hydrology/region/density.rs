use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{
        LAKE_SHORE_OUTER_DISTANCE, LAKE_SHORE_SURFACE_OFFSET, OCEAN_EXTRA_DEPTH,
        OCEAN_MINIMUM_DEPTH, RIVER_CARVE_STRENGTH,
    },
    math::{lerp, ocean_strength, smoothstep},
    types::WaterBody,
};

#[derive(Clone, Copy, Debug)]
struct VerticalDensityDelta {
    minimum_y: f32,
    maximum_y: f32,
    delta: f32,
}

impl VerticalDensityDelta {
    fn at(self, y: f32) -> f32 {
        if y < self.minimum_y || y > self.maximum_y {
            0.0
        } else {
            self.delta
        }
    }
}

#[derive(Debug)]
struct DensityColumnProfile {
    ocean_delta: f32,
    river: Option<VerticalDensityDelta>,
    water_bodies: Vec<VerticalDensityDelta>,
    shore_delta: f32,
}

impl DensityColumnProfile {
    fn delta_at(&self, y: f32) -> f32 {
        let river_delta = self.river.map_or(0.0, |river| river.at(y));
        let carve_delta = self
            .water_bodies
            .iter()
            .copied()
            .map(|body| body.at(y))
            .sum::<f32>();

        self.ocean_delta + river_delta + carve_delta + self.shore_delta
    }
}

impl HydrologyRegion {
    pub(crate) fn density_delta(&self, position: Vec3) -> f32 {
        self.density_column_profile(Vec2::new(position.x, position.z))
            .delta_at(position.y)
    }

    pub(crate) fn density_deltas_for_column<const N: usize>(
        &self,
        horizontal: Vec2,
        first_y: f32,
    ) -> [f32; N] {
        let profile = self.density_column_profile(horizontal);

        std::array::from_fn(|index| profile.delta_at(first_y + index as f32))
    }

    fn density_column_profile(&self, horizontal: Vec2) -> DensityColumnProfile {
        let river_sample = self.river_graph.sample_horizontal(horizontal);
        let (river, river_opening) = river_sample.map_or((None, 0.0), |sample| {
            let profile = smoothstep(sample.strength);
            let bed = sample.height - self.river_carve_depth * profile;
            let river = Some(VerticalDensityDelta {
                minimum_y: bed - 0.5,
                maximum_y: sample.height + 1.5,
                delta: -RIVER_CARVE_STRENGTH * profile,
            });
            let opening = smoothstep((sample.strength * 2.0).clamp(0.0, 1.0));

            (river, opening)
        });

        let macro_sample = self.macro_sample_at(horizontal);
        let ocean_delta = macro_sample.map_or(0.0, |sample| {
            let strength = ocean_strength(sample.continentalness);
            if strength <= 0.0 {
                return 0.0;
            }

            let target_floor =
                self.sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * strength;
            let floor = lerp(sample.elevation, target_floor, strength);

            floor - sample.elevation
        });
        let surface_elevation = macro_sample.map(|sample| sample.elevation);
        let mut water_bodies = Vec::new();

        for body in &self.water_bodies {
            if body.carve_depth <= 0.0 {
                continue;
            }

            let strength = body.horizontal_strength(horizontal);
            if strength <= 0.0 {
                continue;
            }

            let bottom = body.water_level - body.carve_depth * strength;
            water_bodies.push(VerticalDensityDelta {
                minimum_y: bottom - 0.5,
                maximum_y: body.water_level + 0.5,
                delta: -(body.carve_depth * 1.8 + 2.0) * strength,
            });
        }

        let shore_delta = self
            .water_bodies
            .iter()
            .map(|body| {
                lake_shore_density_delta(
                    body,
                    horizontal,
                    river_opening,
                    surface_elevation,
                )
            })
            .max_by(f32::total_cmp)
            .unwrap_or(0.0);

        DensityColumnProfile {
            ocean_delta,
            river,
            water_bodies,
            shore_delta,
        }
    }
}

fn lake_shore_density_delta(
    body: &WaterBody,
    horizontal: Vec2,
    river_opening: f32,
    surface_elevation: Option<f32>,
) -> f32 {
    let shore_strength = lake_shore_strength(body.normalized_horizontal_distance(horizontal));
    if shore_strength <= 0.0 || river_opening >= 1.0 {
        return 0.0;
    }

    let Some(surface_elevation) = surface_elevation else {
        return 0.0;
    };
    let target_surface = body.water_level + LAKE_SHORE_SURFACE_OFFSET;
    let missing_height = (target_surface - surface_elevation).max(0.0);

    missing_height * shore_strength * (1.0 - river_opening)
}

fn lake_shore_strength(distance: f32) -> f32 {
    if distance < 1.0 || distance > LAKE_SHORE_OUTER_DISTANCE {
        return 0.0;
    }

    let progress =
        1.0 - (distance - 1.0) / (LAKE_SHORE_OUTER_DISTANCE - 1.0).max(f32::EPSILON);
    smoothstep(progress.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lake_shore_reinforcement_stays_outside_water_boundary() {
        assert_eq!(lake_shore_strength(0.9), 0.0);
        assert_eq!(lake_shore_strength(1.0), 1.0);
        assert!(lake_shore_strength(1.1) > 0.0);
        assert_eq!(lake_shore_strength(1.3), 0.0);
    }

    #[test]
    fn vertical_density_delta_only_applies_inside_profile() {
        let delta = VerticalDensityDelta {
            minimum_y: 10.0,
            maximum_y: 20.0,
            delta: -4.0,
        };

        assert_eq!(delta.at(9.9), 0.0);
        assert_eq!(delta.at(10.0), -4.0);
        assert_eq!(delta.at(20.0), -4.0);
        assert_eq!(delta.at(20.1), 0.0);
    }
}
