use bevy::prelude::*;

use super::{RiverPath, river_height};
use super::waterfall::{WaterfallLanding, river_path_height, waterfall_profile};
use super::super::super::{
    constants::RIVER_MINIMUM_WATER_DROP,
    drainage::DrainageNode,
    math::{cell_hash, hash_signed, hash_unit, lerp},
};

const RIVER_MEANDER_CONTROL_SPACING: f32 = 34.0;
const RIVER_MEANDER_MIN_INTERVALS: usize = 3;
const RIVER_MEANDER_MAX_INTERVALS: usize = 8;
const RIVER_STRAIGHT_SECTION_CHANCE: f32 = 0.28;

pub(super) fn river_path(
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

#[cfg(test)]
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
}
