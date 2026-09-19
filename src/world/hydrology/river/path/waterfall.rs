use bevy::prelude::*;

use super::RiverPath;
use super::super::super::math::{cell_hash, hash_unit, lerp, smoothstep};

pub(in crate::world::hydrology::river) const WATERFALL_MINIMUM_DROP: f32 = 10.0;
const WATERFALL_MINIMUM_SLOPE: f32 = 0.075;
const WATERFALL_CHANCE: f32 = 0.72;

#[derive(Clone, Copy)]
pub(super) struct WaterfallProfile {
    pub(super) start_t: f32,
    pub(super) end_t: f32,
    pub(super) drop: f32,
}

#[derive(Clone, Copy)]
pub(in crate::world::hydrology::river) struct WaterfallLanding {
    pub(in crate::world::hydrology::river) position: Vec3,
    pub(in crate::world::hydrology::river) drop: f32,
}

pub(super) fn waterfall_profile(
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

pub(super) fn river_path_height(
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

pub(super) fn align_waterfall_landing_to_path(path: &mut RiverPath) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steep_river_drop_can_form_a_waterfall_profile() {
        let profile = waterfall_profile(IVec2::ZERO, 42, 96.0, 110.0, 86.0);

        if let Some(profile) = profile {
            let before = river_path_height(profile.start_t, 110.0, 86.0, Some(profile));
            let after = river_path_height(profile.end_t, 110.0, 86.0, Some(profile));
            assert!(before - after > 110.0 - 86.0 - 8.0);
        }
    }
}
