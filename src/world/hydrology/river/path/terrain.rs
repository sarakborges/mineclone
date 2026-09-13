use bevy::prelude::*;

use super::RIVER_WATER_SURFACE_OFFSET;
use super::super::super::math::lerp;

const RIVER_BANK_SAMPLE_RADIUS_MULTIPLIER: f32 = 1.45;

pub(super) fn constrain_river_path_to_terrain(
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
        let surfaces = [
            surface_elevation_at(horizontal),
            surface_elevation_at(horizontal + bank_offset),
            surface_elevation_at(horizontal - bank_offset),
        ];
        let supported_surface = median_of_three(surfaces);
        let supported_height = (supported_surface - RIVER_WATER_SURFACE_OFFSET).max(1.0);
        let constrained_height = points[index]
            .y
            .min(supported_height)
            .min(upstream_height);

        points[index].y = constrained_height;
        upstream_height = constrained_height;
    }
}

fn median_of_three(mut values: [f32; 3]) -> f32 {
    values.sort_by(f32::total_cmp);
    values[1]
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn one_low_bank_does_not_drag_the_whole_channel_down() {
        let mut points = vec![
            Vec3::new(0.0, 78.0, 0.0),
            Vec3::new(10.0, 77.0, 0.0),
            Vec3::new(20.0, 76.0, 0.0),
        ];

        constrain_river_path_to_terrain(&mut points, 4.0, 4.0, &mut |position| {
            if position.y > 2.0 { 40.0 } else { 80.0 }
        });

        assert!(points[1].y >= 76.0);
    }
}
