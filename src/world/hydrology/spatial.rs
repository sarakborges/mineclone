use bevy::prelude::*;

use super::{
    constants::{HYDROLOGY_REGION_SIZE, LAKE_SHORE_OUTER_DISTANCE, MACRO_SAMPLE_GRID},
    types::WaterBody,
};

// The caller supplies the full footprint of this particular edge, including
// its biome-scaled radius and outer bank. A global maximum river radius would
// discard shore segments before their density contribution reaches a region.
pub(super) fn edge_intersects_region(
    coord: IVec2,
    from: Vec2,
    to: Vec2,
    margin: f32,
) -> bool {
    let (minimum, maximum) = region_bounds(coord);
    let margin = margin.max(0.0);
    let edge_minimum = from.min(to) - Vec2::splat(margin);
    let edge_maximum = from.max(to) + Vec2::splat(margin);

    edge_maximum.x >= minimum.x
        && edge_minimum.x <= maximum.x
        && edge_maximum.y >= minimum.y
        && edge_minimum.y <= maximum.y
}

pub(super) fn water_body_intersects_region(coord: IVec2, body: &WaterBody) -> bool {
    let (minimum, maximum) = region_bounds(coord);
    // Keep exactly the same outer extent as density.rs's column-level cull.
    let extent = Vec2::splat(body.maximum_horizontal_extent() * LAKE_SHORE_OUTER_DISTANCE);
    let body_minimum = body.center - extent;
    let body_maximum = body.center + extent;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::hydrology::constants::RIVER_BANK_OUTER_NORMALIZED_DISTANCE;

    #[test]
    fn region_keeps_river_bank_even_when_channel_itself_cannot_reach_it() {
        let edge = Vec2::new(HYDROLOGY_REGION_SIZE + 20.0, 64.0);
        let radius = 10.0;
        assert!(!edge_intersects_region(IVec2::ZERO, edge, edge, radius));
        assert!(edge_intersects_region(
            IVec2::ZERO,
            edge,
            edge,
            radius * RIVER_BANK_OUTER_NORMALIZED_DISTANCE,
        ));
        let beyond_bank = Vec2::new(HYDROLOGY_REGION_SIZE + 26.0, 64.0);
        assert!(!edge_intersects_region(
            IVec2::ZERO,
            beyond_bank,
            beyond_bank,
            radius * RIVER_BANK_OUTER_NORMALIZED_DISTANCE,
        ));
    }

    #[test]
    fn region_cull_uses_real_biome_scaled_edge_radius() {
        let edge = Vec2::new(HYDROLOGY_REGION_SIZE + 70.0, 64.0);
        assert!(edge_intersects_region(
            IVec2::ZERO,
            edge,
            edge,
            30.0 * RIVER_BANK_OUTER_NORMALIZED_DISTANCE,
        ));
    }

    #[test]
    fn region_keeps_outer_lake_shore_without_including_distant_lakes() {
        let mut body = WaterBody {
            center: Vec2::new(HYDROLOGY_REGION_SIZE + 30.0, 64.0),
            radius: Vec2::splat(20.0),
            rotation: 0.0,
            shape_seed: 42,
            water_level: 90.0,
            carve_depth: 10.0,
            fluid_id: "asteria:water".into(),
        };
        assert!(body.maximum_horizontal_extent() < 30.0);
        assert!(water_body_intersects_region(IVec2::ZERO, &body));
        body.center.x = HYDROLOGY_REGION_SIZE + 35.0;
        assert!(!water_body_intersects_region(IVec2::ZERO, &body));
    }
}
