use bevy::prelude::*;

use super::RIVER_WATER_SURFACE_OFFSET;

pub(super) fn constrain_river_path_to_terrain(
    points: &mut [Vec3],
    _start_radius: f32,
    _end_radius: f32,
    surface_elevation_at: &mut impl FnMut(Vec2) -> f32,
) {
    let last = points.len().saturating_sub(1);
    if last == 0 {
        return;
    }

    let mut upstream_height = points[0].y;

    for point in points.iter_mut() {
        let horizontal = Vec2::new(point.x, point.z);
        let center_surface = surface_elevation_at(horizontal);
        let supported_height = (center_surface - RIVER_WATER_SURFACE_OFFSET).max(1.0);
        let constrained_height = point.y.min(supported_height).min(upstream_height);

        point.y = constrained_height;
        upstream_height = constrained_height;
    }
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
    fn center_channel_can_drop_before_high_banks() {
        let mut points = vec![
            Vec3::new(0.0, 78.0, 0.0),
            Vec3::new(10.0, 77.0, 0.0),
            Vec3::new(20.0, 76.0, 0.0),
        ];

        constrain_river_path_to_terrain(&mut points, 8.0, 8.0, &mut |position| {
            if position.x >= 10.0 { 60.0 } else { 80.0 }
        });

        assert_eq!(points[0].y, 78.0);
        assert_eq!(points[1].y, 58.0);
        assert_eq!(points[2].y, 58.0);
    }
}
