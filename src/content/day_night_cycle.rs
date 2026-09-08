use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::color::Rgb;

#[derive(Clone, Copy, Deserialize)]
pub struct DayNightKeyframe {
    pub time: f32,
    pub sky_tint: Rgb,
    pub fog_tint: Rgb,
    pub ambient_color: Rgb,
    pub ambient_brightness: f32,
    pub sun_color: Rgb,
    pub sun_illuminance: f32,
}

#[derive(Clone, Copy)]
pub struct DayNightSample {
    pub sky_tint: Rgb,
    pub fog_tint: Rgb,
    pub ambient_color: Rgb,
    pub ambient_brightness: f32,
    pub sun_color: Rgb,
    pub sun_illuminance: f32,
}

#[derive(Clone, Deserialize)]
pub struct DayNightCycleDefinition {
    pub id: String,
    pub duration_seconds: f32,
    pub initial_time: f32,
    pub sun_angle_offset_degrees: f32,
    pub keyframes: Vec<DayNightKeyframe>,
}

impl DayNightCycleDefinition {
    pub fn sample(&self, time: f32) -> Option<DayNightSample> {
        let first = *self.keyframes.first()?;
        let last = *self.keyframes.last()?;
        let time = time.clamp(first.time, last.time);

        for pair in self.keyframes.windows(2) {
            let start = pair[0];
            let end = pair[1];

            if time < start.time || time > end.time {
                continue;
            }

            let span = end.time - start.time;
            let t = if span <= f32::EPSILON {
                0.0
            } else {
                (time - start.time) / span
            };

            return Some(interpolate(start, end, t));
        }

        Some(sample_from_keyframe(last))
    }
}

#[derive(Resource, Default)]
pub struct DayNightCycleRegistry {
    definitions: HashMap<String, DayNightCycleDefinition>,
}

impl DayNightCycleRegistry {
    pub fn insert(&mut self, definition: DayNightCycleDefinition) {
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&DayNightCycleDefinition> {
        self.definitions.get(id)
    }
}

fn interpolate(start: DayNightKeyframe, end: DayNightKeyframe, t: f32) -> DayNightSample {
    DayNightSample {
        sky_tint: start.sky_tint.lerp(end.sky_tint, t),
        fog_tint: start.fog_tint.lerp(end.fog_tint, t),
        ambient_color: start.ambient_color.lerp(end.ambient_color, t),
        ambient_brightness: lerp_scalar(start.ambient_brightness, end.ambient_brightness, t),
        sun_color: start.sun_color.lerp(end.sun_color, t),
        sun_illuminance: lerp_scalar(start.sun_illuminance, end.sun_illuminance, t),
    }
}

fn sample_from_keyframe(keyframe: DayNightKeyframe) -> DayNightSample {
    DayNightSample {
        sky_tint: keyframe.sky_tint,
        fog_tint: keyframe.fog_tint,
        ambient_color: keyframe.ambient_color,
        ambient_brightness: keyframe.ambient_brightness,
        sun_color: keyframe.sun_color,
        sun_illuminance: keyframe.sun_illuminance,
    }
}

fn lerp_scalar(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}
