use bevy::prelude::*;

use crate::player::{
    PlayerEntity,
    camera::{CameraPerspective, GameplayCamera},
};

pub(super) const COMPASS_MARKER_COUNT: usize = 24;
pub(super) const COMPASS_MARKER_STEP_DEGREES: f32 = 15.0;
pub(super) const COMPASS_MARKER_WIDTH: f32 = 44.0;
pub(super) const COMPASS_PIXELS_PER_DEGREE: f32 = 2.0;

#[derive(Component)]
pub(super) struct CompassMarker {
    heading_degrees: f32,
}

impl CompassMarker {
    pub(super) fn new(heading_degrees: f32) -> Self {
        Self { heading_degrees }
    }
}

pub(super) fn direction_label(step: usize) -> Option<&'static str> {
    match step {
        0 => Some("N"),
        3 => Some("NE"),
        6 => Some("E"),
        9 => Some("SE"),
        12 => Some("S"),
        15 => Some("SW"),
        18 => Some("W"),
        21 => Some("NW"),
        _ => None,
    }
}

pub(super) fn update_compass_hud(
    camera: Single<&GameplayCamera, With<PlayerEntity>>,
    perspective: Res<CameraPerspective>,
    mut markers: Query<(&CompassMarker, &mut UiTransform)>,
) {
    let heading_degrees = camera_heading_degrees(*camera, *perspective);

    for (marker, mut transform) in &mut markers {
        let delta = wrapped_delta_degrees(marker.heading_degrees, heading_degrees);
        *transform = UiTransform::from_xy(
            px(delta * COMPASS_PIXELS_PER_DEGREE - COMPASS_MARKER_WIDTH * 0.5),
            px(0),
        );
    }
}

fn camera_heading_degrees(camera: &GameplayCamera, perspective: CameraPerspective) -> f32 {
    let yaw = match perspective {
        CameraPerspective::ThirdPersonFront => camera.yaw + std::f32::consts::PI,
        CameraPerspective::FirstPerson | CameraPerspective::ThirdPersonBack => camera.yaw,
    };
    (-yaw.to_degrees()).rem_euclid(360.0)
}

fn wrapped_delta_degrees(target: f32, center: f32) -> f32 {
    (target - center + 180.0).rem_euclid(360.0) - 180.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_uses_minecraft_world_axes() {
        let camera = GameplayCamera::default();
        assert_eq!(
            camera_heading_degrees(&camera, CameraPerspective::FirstPerson),
            0.0
        );

        let east = GameplayCamera::restored(-std::f32::consts::FRAC_PI_2, 0.0);
        assert_eq!(
            camera_heading_degrees(&east, CameraPerspective::FirstPerson),
            90.0
        );
    }

    #[test]
    fn front_camera_reports_the_direction_it_observes() {
        let camera = GameplayCamera::default();
        assert_eq!(
            camera_heading_degrees(&camera, CameraPerspective::ThirdPersonFront),
            180.0
        );
    }

    #[test]
    fn marker_delta_wraps_across_north() {
        assert_eq!(wrapped_delta_degrees(0.0, 350.0), 10.0);
        assert_eq!(wrapped_delta_degrees(350.0, 0.0), -10.0);
    }
}
