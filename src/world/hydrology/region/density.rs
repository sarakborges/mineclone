use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{
        LAKE_SHORE_OUTER_DISTANCE, LAKE_SHORE_SURFACE_OFFSET, OCEAN_EXTRA_DEPTH,
        OCEAN_MINIMUM_DEPTH, RIVER_CARVE_STRENGTH, RIVER_MAXIMUM_RADIUS, SHORE_STRENGTH,
    },
    math::{lerp, ocean_strength, smoothstep},
};

const RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE: f32 = 1.0 - SHORE_STRENGTH;
const RIVER_BANK_OUTER_NORMALIZED_DISTANCE: f32 = 2.5;
const RIVER_BANK_MARGIN: f32 = RIVER_MAXIMUM_RADIUS * 1.5;

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
        self.density_column_profile(Vec2::new(position.x, position.z), None)
            .delta_at(position.y)
    }

    pub(crate) fn density_deltas_for_column<const N: usize>(
        &self,
        horizontal: Vec2,
        first_y: f32,
        actual_surface_height: f32,
    ) -> [f32; N] {
        let profile = self.density_column_profile(horizontal, Some(actual_surface_height));

        std::array::from_fn(|index| profile.delta_at(first_y + index as f32))
    }

    fn density_column_profile(
        &self,
        horizontal: Vec2,
        actual_surface_height: Option<f32>,
    ) -> DensityColumnProfile {
        // Include the shore outside the channel in the same graph scan. The
        // core still uses the original physical radius, not the margin's
        // inflated strength, and the extended ring only grades the terrain.
        let river_graph_sample = self
            .river_graph
            .sample_horizontal_with_margin(horizontal, RIVER_BANK_MARGIN);
        let river_core = river_graph_sample.filter(|sample| sample.normalized_distance < 1.0);
        let (river, river_opening) = river_core.map_or((None, 0.0), |sample| {
            let strength = (1.0 - sample.normalized_distance).clamp(0.0, 1.0);
            let profile = smoothstep(strength);
            let bed = sample.height - self.river_carve_depth * profile;
            let river = (profile > 0.0).then_some(VerticalDensityDelta {
                minimum_y: bed - 0.5,
                maximum_y: sample.height + 1.5,
                delta: -RIVER_CARVE_STRENGTH * profile,
            });
            let opening = smoothstep((strength * 2.0).clamp(0.0, 1.0));

            (river, opening)
        });

        let macro_sample = self.macro_sample_at(horizontal);
        let ocean_strength_at_column = macro_sample.map_or(0.0, |sample| {
            ocean_strength(sample.continentalness, self.ocean_weight)
        });
        let surface_elevation = actual_surface_height.or(macro_sample.map(|sample| sample.elevation));
        let ocean_delta = macro_sample.map_or(0.0, |sample| {
            let strength = ocean_strength_at_column;
            if strength <= 0.0 {
                return 0.0;
            }

            let target_floor =
                self.sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * strength;
            // Keep the existing ocean blend relative to the exact column's
            // starting height. Subtracting a macro floor from an exact surface
            // would create an abrupt jump at the first nonzero ocean strength.
            let base = surface_elevation.unwrap_or(sample.elevation);
            let floor = lerp(base, target_floor, strength);
            floor - base
        });
        let mut water_bodies = Vec::new();
        let mut water_body_opening = 0.0_f32;

        for body in &self.water_bodies {
            let strength = body.horizontal_strength(horizontal);
            water_body_opening = water_body_opening.max(strength);

            if body.carve_depth <= 0.0 || strength <= 0.0 {
                continue;
            }

            let bottom = body.water_level - body.carve_depth * strength;
            water_bodies.push(VerticalDensityDelta {
                minimum_y: bottom - 0.5,
                maximum_y: body.water_level + 0.5,
                delta: -(body.carve_depth * 1.8 + 2.0) * strength,
            });
        }

        let lake_shore_delta = self
            .water_bodies
            .iter()
            .map(|body| {
                shore_density_delta(
                    body.normalized_horizontal_distance(horizontal),
                    body.water_level,
                    river_opening,
                    surface_elevation,
                )
            })
            .max_by(|left, right| left.abs().total_cmp(&right.abs()))
            .unwrap_or(0.0);
        let river_shore_delta = river_graph_sample.map_or(0.0, |sample| {
            shore_density_delta(
                river_shore_normalized_distance(sample.normalized_distance),
                sample.height,
                water_body_opening.max(ocean_strength_at_column),
                surface_elevation,
            )
        });
        let shore_delta = if lake_shore_delta.abs() >= river_shore_delta.abs() {
            lake_shore_delta
        } else {
            river_shore_delta
        };

        DensityColumnProfile {
            ocean_delta,
            river,
            water_bodies,
            shore_delta,
        }
    }
}

fn shore_density_delta(
    normalized_distance: f32,
    water_level: f32,
    opening: f32,
    surface_elevation: Option<f32>,
) -> f32 {
    let shore_strength = shore_strength(normalized_distance);
    if shore_strength <= 0.0 || opening >= 1.0 {
        return 0.0;
    }

    let Some(surface_elevation) = surface_elevation else {
        return 0.0;
    };
    let target_surface = water_level + LAKE_SHORE_SURFACE_OFFSET;
    let height_delta = target_surface - surface_elevation;

    // Remove high terrain above the channel as well as raising a low outer
    // bank. Inside the water footprint only remove roof material: adding
    // positive density there could fill the lake or river itself.
    let height_delta = if normalized_distance < 1.0 {
        height_delta.min(0.0)
    } else {
        height_delta
    };

    height_delta * shore_strength * (1.0 - opening)
}

fn river_shore_normalized_distance(graph_distance: f32) -> f32 {
    if graph_distance <= RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE {
        return graph_distance / RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE;
    }

    let bank_width = (RIVER_BANK_OUTER_NORMALIZED_DISTANCE
        - RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE)
        .max(f32::EPSILON);
    let progress =
        (graph_distance - RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE) / bank_width;

    1.0 + progress * (LAKE_SHORE_OUTER_DISTANCE - 1.0)
}

fn shore_strength(distance: f32) -> f32 {
    if distance <= 1.0 {
        return 1.0;
    }
    if distance >= LAKE_SHORE_OUTER_DISTANCE {
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
    fn shore_grading_removes_high_terrain_above_water() {
        assert!(shore_density_delta(0.9, 90.0, 0.0, Some(120.0)) < 0.0);
        assert!(shore_density_delta(1.0, 90.0, 0.0, Some(120.0)) < 0.0);
        assert!(shore_density_delta(1.1, 90.0, 0.0, Some(120.0)) < 0.0);
    }

    #[test]
    fn low_terrain_is_not_filled_inside_water() {
        assert_eq!(shore_density_delta(0.9, 90.0, 0.0, Some(80.0)), 0.0);
        assert!(shore_density_delta(1.0, 90.0, 0.0, Some(80.0)) > 0.0);
    }

    #[test]
    fn shore_grading_fades_to_unchanged_terrain() {
        assert_eq!(shore_strength(0.9), 1.0);
        assert_eq!(shore_strength(1.0), 1.0);
        assert!(shore_strength(1.1) > 0.0);
        assert_eq!(shore_strength(1.3), 0.0);
    }

    #[test]
    fn river_water_boundary_maps_to_shared_shore_boundary() {
        assert_eq!(
            river_shore_normalized_distance(RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE),
            1.0
        );
    }

    #[test]
    fn river_outer_bank_edge_maps_to_shared_shore_outer_edge() {
        assert!(
            (river_shore_normalized_distance(RIVER_BANK_OUTER_NORMALIZED_DISTANCE)
                - LAKE_SHORE_OUTER_DISTANCE)
                .abs()
                <= f32::EPSILON
        );
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
