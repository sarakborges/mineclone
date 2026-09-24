use bevy::prelude::*;
use smallvec::SmallVec;

use super::{water::bed_has_support, HydrologyRegion};
use crate::world::{
    hydrology::{
        constants::{
            LAKE_SHORE_OUTER_DISTANCE, LAKE_SHORE_SURFACE_OFFSET,
            RIVER_BANK_OUTER_NORMALIZED_DISTANCE, RIVER_CARVE_STRENGTH,
            RIVER_WATER_BODY_APPROACH_MARGIN,
        },
    math::{
        river_channel_profile, smoothstep, RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE,
    },
        types::WaterBody,
    },
};

const INLINE_WATER_BODY_DELTAS: usize = 4;

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
    river: Option<VerticalDensityDelta>,
    // Most columns intersect zero or a few water bodies. Inline storage avoids
    // one allocation per wet column while still supporting arbitrary overlaps.
    water_bodies: SmallVec<[VerticalDensityDelta; INLINE_WATER_BODY_DELTAS]>,
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

        river_delta + carve_delta + self.shore_delta
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
        // Physical channel and dry-bank grading are separate queries.
        // An expanded bank sample must never outrank a real channel from a
        // neighboring/crossing edge, otherwise density can cut dry terrain
        // while the fluid pass follows a different river.
        let river_core = match actual_surface_height {
            Some(surface) => self.river_graph.sample_horizontal_filtered(
                horizontal,
                0.0,
                1.0,
                |sample| {
                    let profile = river_channel_profile(sample.normalized_distance);
                    profile > 0.0
                        && bed_has_support(
                            Some(surface),
                            sample.height - self.river_carve_depth * profile,
                        )
                },
            ),
            None => self.river_graph.sample_horizontal_filtered(
                horizontal,
                0.0,
                1.0,
                |sample| river_channel_profile(sample.normalized_distance) > 0.0,
            ),
        };
        let river_bank = match actual_surface_height {
            Some(surface) => self.river_graph.sample_horizontal_filtered(
                horizontal,
                0.0,
                RIVER_BANK_OUTER_NORMALIZED_DISTANCE,
                |sample| {
                    sample.normalized_distance > RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE
                        && bed_has_support(
                            Some(surface),
                            sample.height - self.river_carve_depth,
                        )
                },
            ),
            None => self.river_graph.sample_horizontal_filtered(
                horizontal,
                0.0,
                RIVER_BANK_OUTER_NORMALIZED_DISTANCE,
                |sample| {
                    sample.normalized_distance > RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE
                },
            ),
        };
        let (mut river, river_opening) = river_core.map_or((None, 0.0), |sample| {
            let profile = river_channel_profile(sample.normalized_distance);
            let bed = sample.height - self.river_carve_depth * profile;
            let carve_surface = actual_surface_height
                .unwrap_or(sample.height + 1.5)
                .max(sample.height + 1.5);
            // A fixed subtraction is not enough when a meander crosses a
            // locally tall column: solid density can survive above the river
            // and become a detached roof/island. Scale the full-strength carve
            // to the actual column relief while preserving the profile fade.
            let required_carve = (carve_surface - bed + 1.0).max(RIVER_CARVE_STRENGTH);
            let river = Some(VerticalDensityDelta {
                minimum_y: bed - 0.5,
                maximum_y: carve_surface + 0.5,
                delta: -required_carve * profile,
            });
            let opening = smoothstep((profile * 2.0).clamp(0.0, 1.0));

            (river, opening)
        });

        let surface_elevation = actual_surface_height;
        let mut water_bodies = SmallVec::new();
        let mut river_water_body_opening = 0.0_f32;
        let mut lake_shore_delta: Option<f32> = None;

        for body in &self.water_bodies {
            // The irregular boundary is bounded by 1.42 * the longest radius.
            // Expand by the entire outer shore before culling: neither water
            // carving nor bank grading can influence more distant columns.
            if !water_body_might_affect_column(body, horizontal) {
                continue;
            }
            // horizontal_strength() computes this exact irregular boundary
            // distance internally. Reuse it for the shore instead of paying
            // for sin/cos and boundary noise a second time per water body.
            let distance = body.normalized_horizontal_distance(horizontal);
            let strength = smoothstep(1.0 - distance.clamp(0.0, 1.0));
            if bed_has_support(
                actual_surface_height,
                body.water_level - body.carve_depth,
            ) {
                river_water_body_opening = river_water_body_opening.max(
                    body.approach_strength_with_margin(
                        horizontal,
                        RIVER_WATER_BODY_APPROACH_MARGIN,
                    ),
                );
            }
            // Reject the lake's inner carving and grading if the original
            // column lies below its bed. Keep outer, dry shore grading: it
            // does not create water, and can still blend a neighboring bank.
            if strength > 0.0
                && !bed_has_support(
                    actual_surface_height,
                    body.water_level - body.carve_depth * strength,
                )
            {
                continue;
            }
            let shore = shore_density_delta(
                distance,
                body.water_level,
                river_opening,
                surface_elevation,
            );
            // Iterator::max_by selects the last item when magnitudes tie.
            // Preserve that order, including signed zero, without another scan.
            if lake_shore_delta.is_none_or(|current| {
                current.abs().total_cmp(&shore.abs()).is_le()
            }) {
                lake_shore_delta = Some(shore);
            }

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

        if let Some(channel) = river.as_mut() {
            let channel_blend = 1.0 - river_water_body_opening.clamp(0.0, 1.0);
            channel.delta *= channel_blend;
            if channel_blend <= f32::EPSILON {
                river = None;
            }
        }

        let lake_shore_delta = lake_shore_delta.unwrap_or(0.0);
        let river_shore_delta = if river_core.is_some() {
            0.0
        } else {
            river_bank.map_or(0.0, |sample| {
                shore_density_delta(
                    river_shore_normalized_distance(sample.normalized_distance),
                    sample.height,
                    river_water_body_opening,
                    surface_elevation,
                )
            })
        };
        let shore_delta = if lake_shore_delta.abs() >= river_shore_delta.abs() {
            lake_shore_delta
        } else {
            river_shore_delta
        };

        DensityColumnProfile {
            river,
            water_bodies,
            shore_delta,
        }
    }
}

fn water_body_might_affect_column(body: &WaterBody, position: Vec2) -> bool {
    let shore_extent = body.maximum_horizontal_extent() * LAKE_SHORE_OUTER_DISTANCE;
    let approach_extent =
        (body.radius.max_element() + RIVER_WATER_BODY_APPROACH_MARGIN) * 1.42;
    let outer_extent = shore_extent.max(approach_extent);
    body.center.distance_squared(position) <= outer_extent * outer_extent
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

    // Shore grading is subtractive only. Positive density here used to
    // manufacture isolated blocks above rivers and could build a wall where a
    // river met a lake. Natural hydrology may lower a high bank, never raise
    // terrain that was not present in the original column.
    height_delta.min(0.0) * shore_strength * (1.0 - opening)
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
    use crate::{
        content::dimension_hydrology::DimensionHydrology,
        world::feature_graph::FeatureGraph,
    };

    fn test_region(river_graph: FeatureGraph, water_bodies: Vec<WaterBody>) -> HydrologyRegion {
        HydrologyRegion {
            river_graph,
            river_carve_depth: 7.0,
            water_bodies,
            settings: DimensionHydrology::default(),
        }
    }

    #[test]
    fn unsupported_lake_does_not_carve_a_dry_basin() {
        let body = WaterBody {
            center: Vec2::ZERO,
            radius: Vec2::splat(20.0),
            rotation: 0.0,
            shape_seed: 42,
            water_level: 100.0,
            carve_depth: 6.0,
            fluid_id: "asteria:test/water".into(),
        };
        let region = test_region(FeatureGraph::default(), vec![body]);
        assert!(region.water_at(Vec2::ZERO).is_some());
        assert!(region.supported_water_at(Vec2::ZERO, 85.0).is_none());
        assert_eq!(
            region.density_deltas_for_column::<1>(Vec2::ZERO, 98.5, 85.0),
            [0.0]
        );
        assert!(region.density_deltas_for_column::<1>(Vec2::ZERO, 98.5, 95.0)[0] < 0.0);
    }

    #[test]
    fn unsupported_river_does_not_carve_a_dry_channel_or_headroom() {
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::new(0.0, 100.0, 0.0));
        let to = graph.add_node(Vec3::new(20.0, 100.0, 0.0));
        graph.add_edge(from, to, 8.0, 8.0);
        let region = test_region(graph, Vec::new());
        let center = Vec2::new(10.0, 0.0);
        assert!(region.river_surface_at(center).is_some());
        assert!(region.supported_river_surface_at(center, 80.0).is_none());
        assert!(region.supported_river_surface_at(center, 94.0).is_some());
        assert_eq!(
            region.density_deltas_for_column::<1>(center, 99.5, 80.0),
            [0.0]
        );
        assert!(region.density_deltas_for_column::<1>(center, 99.5, 94.0)[0] < 0.0);
    }

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
    fn dry_ring_outside_river_water_does_not_get_a_channel_profile() {
        let boundary = RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE;
        assert!(river_channel_profile(boundary - 0.01) > 0.0);
        assert_eq!(river_channel_profile(boundary), 0.0);
        assert_eq!(river_channel_profile(0.9), 0.0);
        assert!(river_shore_normalized_distance(0.9) > 1.0);
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

    #[test]
    fn river_carve_fades_out_inside_lake_basin() {
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::new(-20.0, 90.0, 0.0));
        let to = graph.add_node(Vec3::new(20.0, 90.0, 0.0));
        graph.add_edge(from, to, 8.0, 8.0);
        let body = WaterBody {
            center: Vec2::ZERO,
            radius: Vec2::splat(24.0),
            rotation: 0.0,
            shape_seed: 42,
            water_level: 90.0,
            carve_depth: 10.0,
            fluid_id: "asteria:test/water".into(),
        };
        let with_river = test_region(graph, vec![body.clone()]);
        let lake_only = test_region(FeatureGraph::default(), vec![body]);

        let position = Vec2::ZERO;
        let y = 84.5;
        let river_lake =
            with_river.density_deltas_for_column::<1>(position, y, 120.0)[0];
        let lake =
            lake_only.density_deltas_for_column::<1>(position, y, 120.0)[0];

        assert!((river_lake - lake).abs() < 0.001);
    }

    #[test]
    fn water_overlap_storage_handles_more_bodies_than_inline_capacity() {
        let mut bodies = SmallVec::<[VerticalDensityDelta; INLINE_WATER_BODY_DELTAS]>::new();
        for index in 1..=6 {
            bodies.push(VerticalDensityDelta {
                minimum_y: 10.0,
                maximum_y: 20.0,
                delta: -(index as f32),
            });
        }
        let profile = DensityColumnProfile {
            river: None,
            water_bodies: bodies,
            shore_delta: 0.0,
        };

        assert_eq!(profile.water_bodies.len(), 6);
        assert_eq!(profile.delta_at(15.0), -21.0);
        assert_eq!(profile.delta_at(25.0), 0.0);
    }

    #[test]
    fn shared_boundary_distance_preserves_water_strength() {
        let body = WaterBody {
            center: Vec2::ZERO,
            radius: Vec2::splat(20.0),
            rotation: 0.4,
            shape_seed: 42,
            water_level: 90.0,
            carve_depth: 10.0,
            fluid_id: "asteria:water".into(),
        };
        for position in [Vec2::ZERO, Vec2::new(8.0, 5.0), Vec2::new(30.0, 0.0)] {
            let distance = body.normalized_horizontal_distance(position);
            let strength = smoothstep(1.0 - distance.clamp(0.0, 1.0));
            assert_eq!(strength, body.horizontal_strength(position));
        }
    }

    #[test]
    fn distant_water_body_cull_keeps_the_entire_shore_margin() {
        let body = WaterBody {
            center: Vec2::ZERO,
            radius: Vec2::new(20.0, 12.0),
            rotation: 0.4,
            shape_seed: 42,
            water_level: 90.0,
            carve_depth: 10.0,
            fluid_id: "asteria:water".into(),
        };
        let outer_extent = body.maximum_horizontal_extent() * LAKE_SHORE_OUTER_DISTANCE;
        assert!(water_body_might_affect_column(&body, Vec2::ZERO));
        assert!(water_body_might_affect_column(&body, Vec2::new(20.0, 0.0)));
        let distant = Vec2::new(outer_extent + 1.0, 0.0);
        assert!(!water_body_might_affect_column(&body, distant));
        assert!(body.normalized_horizontal_distance(distant) > LAKE_SHORE_OUTER_DISTANCE);
        assert_eq!(body.horizontal_strength(distant), 0.0);
        assert_eq!(shore_strength(body.normalized_horizontal_distance(distant)), 0.0);
    }
}
