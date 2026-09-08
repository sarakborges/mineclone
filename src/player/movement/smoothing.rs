use bevy::prelude::*;

pub(super) fn approach_velocity(current: Vec3, target: Vec3, max_delta: f32) -> Vec3 {
    let delta = target - current;
    let distance = delta.length();

    if distance <= max_delta || distance <= f32::EPSILON {
        target
    } else {
        current + delta / distance * max_delta
    }
}
