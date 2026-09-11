use bevy::prelude::*;

use crate::world::feature_graph::FeatureGraph;

use super::super::{
    constants::{
        OCEAN_CONTINENTALNESS_THRESHOLD, RIVER_FLOW_FOR_MAX_WIDTH, RIVER_MAXIMUM_RADIUS,
        RIVER_MINIMUM_FLOW, RIVER_MINIMUM_RADIUS, RIVER_MINIMUM_WATER_DROP,
    },
    drainage::DrainageNode,
    math::{cell_hash, hash_signed, hash_unit, lerp, smoothstep},
    spatial::edge_intersects_region,
};

const RIVER_WATER_SURFACE_OFFSET: f32 = 2.0;
const RIVER_BANK_SAMPLE_RADIUS_MULTIPLIER: f32 = 1.15;
const RIVER_MEANDER_CONTROL_SPACING: f32 = 34.0;
const RIVER_MEANDER_MIN_INTERVALS: usize = 3;
const RIVER_MEANDER_MAX_INTERVALS: usize = 8;
const RIVER_STRAIGHT_SECTION_CHANCE: f32 = 0.28;
pub(super) const WATERFALL_MINIMUM_DROP: f32 = 10.0;
const WATERFALL_MINIMUM_SLOPE: f32 = 0.075;
const WATERFALL_CHANCE: f32 = 0.72;

#[derive(Clone, Copy)]
pub(super) struct RiverEdgeSpec {
    pub(super) region_coord: IVec2,
    pub(super) source_cell: IVec2,
    pub(super) source: DrainageNode,
    pub(super) downstream: DrainageNode,
    pub(super) source_water_level: Option<f32>,
    pub(super) downstream_water_level: Option<f32>,
    pub(super) flow: u32,
    pub(super) downstream_flow: u32,
    pub(super) seed: u64,
    pub(super) sea_level: f32,
}

struct RiverPath {
    points: Vec<Vec3>,
    waterfall: Option<WaterfallLanding>,
}

#[derive(Clone, Copy)]
struct WaterfallProfile {
    start_t: f32,
    end_t: f32,
    drop: f32,
}

#[derive(Clone, Copy)]
pub(super) struct WaterfallLanding {
    pub(super) position: Vec3,
    pub(super) drop: f32,
}

pub(super) fn add_curved_river_edge<F>(
    graph: &mut FeatureGraph,
    spec: RiverEdgeSpec,
    mut surface_elevation_at: F,
) -> Option<WaterfallLanding>
where
    F: FnMut(Vec2) -> f32,
{
    let width_multiplier = (spec.source.biome_hydrology.river_width_multiplier
        + spec.downstream.biome_hydrology.river_width_multiplier)
        * 0.5;
    let start_radius = river_radius(spec.flow) * width_multiplier;
    let end_radius = river_radius(spec.downstream_flow.max(spec.flow)) * width_multiplier;

    if start_radius <= f32::EPSILON || end_radius <= f32::EPSILON {
        return None;
    }

    let mut path = river_path(
        spec.source_cell,
        spec.source,
        spec.downstream,
        spec.seed,
        spec.sea_level,
        spec.source_water_level,
        spec.downstream_water_level,
    );
    constrain_river_path_to_terrain(
        &mut path.points,
        start_radius,
        end_radius,
        &mut surface_elevation_at,
    );
    align_waterfall_landing_to_path(&mut path);
    let last = path.points.len().saturating_sub(1);

    for index in 0..last {
        let from = path.points[index];
        let to = path.points[index + 1];
        if !edge_intersects_region(
            spec.region_coord,
            Vec2::new(from.x, from.z),
            Vec2::new(to.x, to.z),
        ) {
            continue;
        }

        let from_t = index as f32 / last as f32;
        let to_t = (index + 1) as f32 / last as f32;
        let from_radius = lerp(start_radius, end_radius, from_t);
        let to_radius = lerp(start_radius, end_radius, to_t);
        let from_node = graph.add_node(from);
        let to_node = graph.add_node(to);
        graph.add_edge(from_node, to_node, from_radius, to_radius);
    }

    path.waterfall
}

fn constrain_river_path_to_terrain(
    points: &mut [Vec3],
    start_radius: f32,
    end_radius: f32,
    surface_elevation_at: &mut impl FnMut(Vec2) -> f32,
) {
    let last = points.len().saturating_sub(1);
    if last == 0 {
        return;
    }

    let mut upstream_height = points[0].y;

    for index in 0..=last {
        let t = index as f32 / last as f32;
        let horizontal = Vec2::new(points[index].x, points[index].z);
        let tangent = if index == 0 {
            points[1] - points[0]
        } else if index == last {
            points[last] - points[last - 1]
        } else {
            points[index + 1] - points[index - 1]
        };
        let horizontal_tangent = Vec2::new(tangent.x, tangent.z).normalize_or_zero();
        let bank_normal = Vec2::new(-horizontal_tangent.y, horizontal_tangent.x);
        let radius = lerp(start_radius, end_radius, t);
        let bank_offset = bank_normal * radius * RIVER_BANK_SAMPLE_RADIUS_MULTIPLIER;
        let supported_surface = [horizontal, horizontal + bank_offset, horizontal - bank_offset]
            .into_iter()
            .map(&mut *surface_elevation_at)
            .min_by(f32::total_cmp)
            .unwrap_or(points[index].y + RIVER_WATER_SURFACE_OFFSET);
        let supported_height = (supported_surface - RIVER_WATER_SURFACE_OFFSET).max(1.0);
        let constrained_height = points[index]
            .y
            .min(supported_height)
            .min(upstream_height);

        points[index].y = constrained_height;
        upstream_height = constrained_height;
    }
}

fn align_waterfall_landing_to_path(path: &mut RiverPath) {
    let Some(waterfall) = path.waterfall.as_mut() else {
        return;
    };
    let target = Vec2::new(waterfall.position.x, waterfall.position.z);
    let Some(point) = path.points.iter().min_by(|left, right| {
        Vec2::new(left.x, left.z)
            .distance_squared(target)
            .total_cmp(&Vec2::new(right.x, right.z).distance_squared(target))
    }) else {
        return;
    };

    waterfall.position = *point;
}

fn river_radius(flow: u32) -> f32 {
    let minimum = RIVER_MINIMUM_FLOW as f32;
    let normalized = if flow as f32 <= minimum {
        0.0
    } else {
        ((flow as f32 / minimum).ln() / (RIVER_FLOW_FOR_MAX_WIDTH / minimum).ln()).clamp(0.0, 1.0)
    };

    lerp(RIVER_MINIMUM_RADIUS, RIVER_MAXIMUM_RADIUS, normalized)
}

fn river_path(
    source_cell: IVec2,
    source: DrainageNode,
    downstream: DrainageNode,
    seed: u64,
    sea_level: f32,
    source_water_level: Option<f32>,
    downstream_water_level: Option<f32>,
) -> RiverPath {
    let delta = downstream.position - source.position;
    let distance = delta.length();
    let segment_count = ((distance / 10.0).ceil() as usize).clamp(7, 26);
    let direction = delta.normalize_or_zero();
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let lateral_controls = river_lateral_controls(source_cell, seed, distance);
    let start_height = source_water_level.unwrap_or_else(|| river_height(source, sea_level));
    let raw_end_height =
        downstream_water_level.unwrap_or_else(|| river_height(downstream, sea_level));
    let end_height = if downstream_water_level.is_some() {
        raw_end_height.min(start_height).max(1.0)
    } else {
        raw_end_height
            .min(start_height - RIVER_MINIMUM_WATER_DROP)
            .max(1.0)
    };
    let waterfall_profile = waterfall_profile(
        source_cell,
        seed,
        distance,
        start_height,
        end_height,
    );
    let points = (0..=segment_count)
        .map(|index| {
            let t = index as f32 / segment_count as f32;
            let lateral = sample_lateral_controls(&lateral_controls, t);
            let horizontal = source.position.lerp(downstream.position, t) + perpendicular * lateral;
            let height = river_path_height(t, start_height, end_height, waterfall_profile);

            Vec3::new(horizontal.x, height, horizontal.y)
        })
        .collect::<Vec<_>>();
    let waterfall = waterfall_profile.map(|profile| {
        let landing_index =
            ((profile.end_t * segment_count as f32).round() as usize).min(segment_count);
        WaterfallLanding {
            position: points[landing_index],
            drop: profile.drop,
        }
    });

    RiverPath { points, waterfall }
}

fn river_path_points(
    source_cell: IVec2,
    source: DrainageNode,
    downstream: DrainageNode,
    seed: u64,
    sea_level: f32,
) -> Vec<Vec3> {
    river_path(
        source_cell,
        source,
        downstream,
        seed,
        sea_level,
        None,
        None,
    )
    .points
}

fn waterfall_profile(
    source_cell: IVec2,
    seed: u64,
    distance: f32,
    start_height: f32,
    end_height: f32,
) -> Option<WaterfallProfile> {
    let drop = start_height - end_height;
    if distance <= f32::EPSILON
        || drop < WATERFALL_MINIMUM_DROP
        || drop / distance < WATERFALL_MINIMUM_SLOPE
    {
        return None;
    }

    let hash = cell_hash(source_cell, seed ^ 0x5be0_cd19_137e_2179);
    if hash_unit(hash.rotate_left(9)) >= WATERFALL_CHANCE {
        return None;
    }

    let center = lerp(0.42, 0.68, hash_unit(hash.rotate_left(23)));
    let horizontal_span = lerp(5.0, 10.0, hash_unit(hash.rotate_left(37)));
    let half_width = (horizontal_span / distance * 0.5).clamp(0.025, 0.09);

    Some(WaterfallProfile {
        start_t: (center - half_width).clamp(0.18, 0.78),
        end_t: (center + half_width).clamp(0.22, 0.84),
        drop,
    })
}

fn river_path_height(
    t: f32,
    start_height: f32,
    end_height: f32,
    waterfall: Option<WaterfallProfile>,
) -> f32 {
    let Some(waterfall) = waterfall else {
        return lerp(start_height, end_height, smoothstep(t));
    };

    let approach_height = start_height - waterfall.drop * 0.14;
    let landing_height = end_height + waterfall.drop * 0.10;

    if t <= waterfall.start_t {
        let progress = (t / waterfall.start_t).clamp(0.0, 1.0);
        return lerp(start_height, approach_height, smoothstep(progress));
    }
    if t <= waterfall.end_t {
        let progress = ((t - waterfall.start_t) / (waterfall.end_t - waterfall.start_t))
            .clamp(0.0, 1.0);
        return lerp(approach_height, landing_height, smoothstep(progress));
    }

    let progress = ((t - waterfall.end_t) / (1.0 - waterfall.end_t)).clamp(0.0, 1.0);
    lerp(landing_height, end_height, smoothstep(progress))
}

fn river_lateral_controls(source_cell: IVec2, seed: u64, distance: f32) -> Vec<f32> {
    let interval_count = ((distance / RIVER_MEANDER_CONTROL_SPACING).ceil() as usize)
        .clamp(RIVER_MEANDER_MIN_INTERVALS, RIVER_MEANDER_MAX_INTERVALS);
    let base_hash = cell_hash(source_cell, seed ^ 0x6a09_e667_f3bc_c909);
    let amplitude = (distance * lerp(0.10, 0.28, hash_unit(base_hash))).clamp(7.0, 40.0);
    let mut controls = Vec::with_capacity(interval_count + 1);

    controls.push(0.0);

    for index in 1..interval_count {
        let index_seed = seed
            ^ 0xbb67_ae85_84ca_a73b
            ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let control_hash = cell_hash(source_cell, index_seed ^ base_hash.rotate_left(17));
        let straight_section =
            hash_unit(control_hash.rotate_left(13)) < RIVER_STRAIGHT_SECTION_CHANCE;
        let strength = if straight_section {
            lerp(0.05, 0.22, hash_unit(control_hash.rotate_left(29)))
        } else {
            lerp(0.55, 1.0, hash_unit(control_hash.rotate_left(43)))
        };
        let lateral = hash_signed(control_hash.rotate_left(7)) * amplitude * strength;

        controls.push(lateral);
    }

    controls.push(0.0);
    controls
}

fn sample_lateral_controls(controls: &[f32], t: f32) -> f32 {
    let interval_count = controls.len().saturating_sub(1);
    if interval_count == 0 {
        return 0.0;
    }

    let scaled = t.clamp(0.0, 1.0) * interval_count as f32;
    let index = (scaled.floor() as usize).min(interval_count - 1);
    let local_t = (scaled - index as f32).clamp(0.0, 1.0);
    let p0 = controls[index.saturating_sub(1)];
    let p1 = controls[index];
    let p2 = controls[(index + 1).min(interval_count)];
    let p3 = controls[(index + 2).min(interval_count)];

    catmull_rom(p0, p1, p2, p3, local_t)
}

fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;

    0.5 * (2.0 * p1
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

pub(super) fn river_height(node: DrainageNode, sea_level: f32) -> f32 {
    if node.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD {
        sea_level
    } else {
        (node.elevation - RIVER_WATER_SURFACE_OFFSET).max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::biome_hydrology::BiomeHydrology;

    fn node(position: Vec2, elevation: f32) -> DrainageNode {
        DrainageNode {
            position,
            elevation,
            continentalness: 0.8,
            biome_hydrology: BiomeHydrology::default(),
        }
    }

    #[test]
    fn river_width_increases_with_flow() {
        assert!(river_radius(RIVER_MINIMUM_FLOW * 3) > river_radius(RIVER_MINIMUM_FLOW));
    }

    #[test]
    fn river_paths_meander_between_exact_endpoints() {
        let source = node(Vec2::ZERO, 100.0);
        let downstream = node(Vec2::new(128.0, 0.0), 92.0);
        let points = river_path_points(IVec2::ZERO, source, downstream, 42, 64.0);

        assert_eq!(points.first().unwrap().x, source.position.x);
        assert_eq!(points.first().unwrap().z, source.position.y);
        assert_eq!(points.last().unwrap().x, downstream.position.x);
        assert_eq!(points.last().unwrap().z, downstream.position.y);
        assert!(
            points[1..points.len() - 1]
                .iter()
                .any(|point| point.z != 0.0)
        );
    }

    #[test]
    fn river_paths_are_deterministic() {
        let source = node(Vec2::ZERO, 100.0);
        let downstream = node(Vec2::new(128.0, 0.0), 92.0);

        assert_eq!(
            river_path_points(IVec2::new(4, -7), source, downstream, 42, 64.0),
            river_path_points(IVec2::new(4, -7), source, downstream, 42, 64.0),
        );
    }

    #[test]
    fn river_surface_drops_downstream() {
        let source = node(Vec2::ZERO, 80.0);
        let downstream = node(Vec2::new(128.0, 0.0), 79.9);
        let points = river_path_points(IVec2::ZERO, source, downstream, 42, 64.0);
        let start = points.first().unwrap().y;
        let end = points.last().unwrap().y;

        assert!(start - end >= RIVER_MINIMUM_WATER_DROP);
    }

    #[test]
    fn lake_endpoints_preserve_their_water_surface() {
        let source = node(Vec2::ZERO, 96.0);
        let downstream = node(Vec2::new(128.0, 0.0), 82.0);
        let source_lake_level = 95.35;
        let downstream_lake_level = 81.35;
        let path = river_path(
            IVec2::ZERO,
            source,
            downstream,
            42,
            64.0,
            Some(source_lake_level),
            Some(downstream_lake_level),
        );

        assert_eq!(path.points.first().unwrap().y, source_lake_level);
        assert_eq!(path.points.last().unwrap().y, downstream_lake_level);
    }

    #[test]
    fn terrain_support_prevents_floating_and_uphill_recovery() {
        let mut points = vec![
            Vec3::new(0.0, 78.0, 0.0),
            Vec3::new(10.0, 77.0, 0.0),
            Vec3::new(20.0, 76.0, 0.0),
        ];

        constrain_river_path_to_terrain(&mut points, 4.0, 4.0, &mut |position| {
            if position.x < 15.0 { 80.0 } else { 50.0 }
        });

        assert_eq!(points[0].y, 78.0);
        assert_eq!(points[1].y, 77.0);
        assert_eq!(points[2].y, 48.0);

        let mut recovered = vec![
            Vec3::new(0.0, 78.0, 0.0),
            Vec3::new(10.0, 77.0, 0.0),
            Vec3::new(20.0, 76.0, 0.0),
        ];
        constrain_river_path_to_terrain(&mut recovered, 4.0, 4.0, &mut |position| {
            if position.x < 5.0 || position.x > 15.0 { 80.0 } else { 50.0 }
        });

        assert_eq!(recovered[1].y, 48.0);
        assert_eq!(recovered[2].y, 48.0);
    }

    #[test]
    fn steep_river_drop_can_form_a_waterfall_profile() {
        let profile = waterfall_profile(IVec2::ZERO, 42, 96.0, 110.0, 86.0);

        if let Some(profile) = profile {
            let before = river_path_height(profile.start_t, 110.0, 86.0, Some(profile));
            let after = river_path_height(profile.end_t, 110.0, 86.0, Some(profile));
            assert!(before - after > 110.0 - 86.0 - 8.0);
        }
    }

    #[test]
    fn river_surface_stays_two_blocks_below_land_sample() {
        let source = node(Vec2::ZERO, 80.0);

        assert_eq!(river_height(source, 64.0), 78.0);
    }
}
