use bevy::prelude::*;

use crate::content::{
    day_night_cycle::DayNightCycleDefinition,
    sky::CelestialBodyDefinition,
};

pub fn celestial_direction(
    definition: &CelestialBodyDefinition,
    cycle: &DayNightCycleDefinition,
    normalized_time: f32,
) -> Option<Vec3> {
    let progress = cycle.progress_between_phases(
        normalized_time,
        definition.rise_phase,
        definition.set_phase,
    )?;
    let azimuth = lerp_angle(
        definition.rise_azimuth_degrees.to_radians(),
        definition.set_azimuth_degrees.to_radians(),
        progress,
    );
    let altitude = (progress * std::f32::consts::PI).sin()
        * definition.max_altitude_degrees.to_radians();
    let horizontal_radius = altitude.cos();

    Some(
        Vec3::new(
            azimuth.sin() * horizontal_radius,
            altitude.sin(),
            -azimuth.cos() * horizontal_radius,
        )
        .normalize(),
    )
}

pub fn celestial_offset(
    definition: &CelestialBodyDefinition,
    cycle: &DayNightCycleDefinition,
    normalized_time: f32,
) -> Option<Vec3> {
    celestial_direction(definition, cycle, normalized_time)
        .map(|direction| direction * definition.orbit_radius)
}

fn lerp_angle(start: f32, end: f32, t: f32) -> f32 {
    let delta = (end - start + std::f32::consts::PI)
        .rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    start + delta * t
}
