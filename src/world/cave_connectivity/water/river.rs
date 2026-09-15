use bevy::prelude::*;

use crate::world::{
    feature_graph::FeatureGraph,
    math::{lerp, smoothstep},
};

use super::{
    UndergroundWaterSample,
    hash::{hash_unit, pair_hash},
    lake::anchor_has_lake,
};

const UNDERGROUND_RIVER_CHANCE: f32 = 0.28;
const UNDERGROUND_LAKE_CONNECTION_CHANCE: f32 = 0.86;
const UNDERGROUND_RIVER_RADIUS_SCALE: f32 = 0.48;
const UNDERGROUND_RIVER_MINIMUM_RADIUS: f32 = 1.25;
const UNDERGROUND_RIVER_MAXIMUM_RADIUS: f32 = 4.0;
const UNDERGROUND_RIVER_SURFACE_RADIUS_OFFSET: f32 = 0.38;
const UNDERGROUND_RIVER_MINIMUM_SURFACE_OFFSET: f32 = 1.0;
const UNDERGROUND_RIVER_MINIMUM_DEPTH: f32 = 0.75;
const UNDERGROUND_RIVER_MAXIMUM_DEPTH: f32 = 2.75;
const UNDERGROUND_WATERFALL_MINIMUM_DROP: f32 = 4.0;
const UNDERGROUND_WATERFALL_MINIMUM_SLOPE: f32 = 0.42;
const UNDERGROUND_WATERFALL_RADIUS_SCALE: f32 = 0.62;
const UNDERGROUND_WATERFALL_MINIMUM_RADIUS: f32 = 1.0;
const UNDERGROUND_WATERFALL_MAXIMUM_RADIUS: f32 = 3.25;

#[derive(Clone, Copy, Debug)]
pub(super) struct UndergroundWaterfall {
    pub(super) from: Vec3,
    pub(super) to: Vec3,
    radius: f32,
}

impl UndergroundWaterfall {
    pub(super) fn contains(&self, position: Vec3) -> bool {
        let segment = self.to - self.from;
        let length_squared = segment.length_squared();
        if length_squared <= f32::EPSILON {
            return false;
        }

        let progress = ((position - self.from).dot(segment) / length_squared).clamp(0.0, 1.0);
        let closest = self.from + segment * progress;
        position.distance(closest) <= self.radius
    }
}

pub(super) fn connection_carries_water(from: Vec3, to: Vec3, seed: u64) -> bool {
    let from_lake = anchor_has_lake(from, seed);
    let to_lake = anchor_has_lake(to, seed);
    let high_lake_drains_down =
        (from_lake && from.y > to.y + 2.0) || (to_lake && to.y > from.y + 2.0);
    if high_lake_drains_down {
        return true;
    }

    let hash = pair_hash(from, to, seed ^ 0x510e_527f_ade6_82d1);
    let chance = if from_lake || to_lake {
        UNDERGROUND_LAKE_CONNECTION_CHANCE
    } else {
        UNDERGROUND_RIVER_CHANCE
    };

    hash_unit(hash.rotate_left(17)) < chance
}

pub(super) fn add_river_segment(
    river_graph: &mut FeatureGraph,
    waterfalls: &mut Vec<UndergroundWaterfall>,
    from: Vec3,
    to: Vec3,
    start_cave_radius: f32,
    end_cave_radius: f32,
) {
    let start_surface_offset = (start_cave_radius * UNDERGROUND_RIVER_SURFACE_RADIUS_OFFSET)
        .max(UNDERGROUND_RIVER_MINIMUM_SURFACE_OFFSET);
    let end_surface_offset = (end_cave_radius * UNDERGROUND_RIVER_SURFACE_RADIUS_OFFSET)
        .max(UNDERGROUND_RIVER_MINIMUM_SURFACE_OFFSET);
    let river_from = from - Vec3::Y * start_surface_offset;
    let river_to = to - Vec3::Y * end_surface_offset;
    let start_radius = (start_cave_radius * UNDERGROUND_RIVER_RADIUS_SCALE)
        .clamp(UNDERGROUND_RIVER_MINIMUM_RADIUS, UNDERGROUND_RIVER_MAXIMUM_RADIUS);
    let end_radius = (end_cave_radius * UNDERGROUND_RIVER_RADIUS_SCALE)
        .clamp(UNDERGROUND_RIVER_MINIMUM_RADIUS, UNDERGROUND_RIVER_MAXIMUM_RADIUS);
    let from_node = river_graph.add_node(river_from);
    let to_node = river_graph.add_node(river_to);

    river_graph.add_edge(from_node, to_node, start_radius, end_radius);

    let vertical_drop = (river_from.y - river_to.y).abs();
    let horizontal_distance = Vec2::new(
        river_to.x - river_from.x,
        river_to.z - river_from.z,
    )
    .length()
    .max(0.5);
    let slope = vertical_drop / horizontal_distance;
    if vertical_drop >= UNDERGROUND_WATERFALL_MINIMUM_DROP
        && slope >= UNDERGROUND_WATERFALL_MINIMUM_SLOPE
    {
        let radius = (start_radius.max(end_radius) * UNDERGROUND_WATERFALL_RADIUS_SCALE).clamp(
            UNDERGROUND_WATERFALL_MINIMUM_RADIUS,
            UNDERGROUND_WATERFALL_MAXIMUM_RADIUS,
        );
        let (high, low) = if river_from.y >= river_to.y {
            (river_from, river_to)
        } else {
            (river_to, river_from)
        };
        waterfalls.push(UndergroundWaterfall {
            from: high,
            to: low,
            radius,
        });
    }
}

pub(super) fn river_sample_at(
    river_graph: &FeatureGraph,
    horizontal: Vec2,
) -> Option<UndergroundWaterSample> {
    let river = river_graph.sample_horizontal(horizontal)?;
    let strength = smoothstep(river.strength.clamp(0.0, 1.0));

    Some(UndergroundWaterSample {
        water_level: river.height,
        bed_level: river.height
            - lerp(
                UNDERGROUND_RIVER_MINIMUM_DEPTH,
                UNDERGROUND_RIVER_MAXIMUM_DEPTH,
                strength,
            ),
    })
}
