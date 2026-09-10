use bevy::prelude::*;

use super::{
    constants::{HYDROLOGY_REGION_SIZE, MACRO_SAMPLE_GRID, RIVER_MAXIMUM_RADIUS},
    types::WaterBody,
};

pub(super) fn edge_intersects_region(coord: IVec2, from: Vec2, to: Vec2) -> bool {
    let (minimum, maximum) = region_bounds(coord);
    let margin = RIVER_MAXIMUM_RADIUS;
    let edge_minimum = from.min(to) - Vec2::splat(margin);
    let edge_maximum = from.max(to) + Vec2::splat(margin);

    edge_maximum.x >= minimum.x
        && edge_minimum.x <= maximum.x
        && edge_maximum.y >= minimum.y
        && edge_minimum.y <= maximum.y
}

pub(super) fn water_body_intersects_region(coord: IVec2, body: &WaterBody) -> bool {
    let (minimum, maximum) = region_bounds(coord);
    let body_minimum = body.center - body.radius;
    let body_maximum = body.center + body.radius;

    body_maximum.x >= minimum.x
        && body_minimum.x <= maximum.x
        && body_maximum.y >= minimum.y
        && body_minimum.y <= maximum.y
}

fn region_bounds(coord: IVec2) -> (Vec2, Vec2) {
    let minimum = coord.as_vec2() * HYDROLOGY_REGION_SIZE;
    (minimum, minimum + Vec2::splat(HYDROLOGY_REGION_SIZE))
}

pub(super) fn macro_sample_position(coord: IVec2, x: usize, z: usize) -> Vec2 {
    let origin = coord.as_vec2() * HYDROLOGY_REGION_SIZE;
    let step = HYDROLOGY_REGION_SIZE / (MACRO_SAMPLE_GRID - 1) as f32;

    origin + Vec2::new(x as f32 * step, z as f32 * step)
}
