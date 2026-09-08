use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::{
    color::Rgb,
    day_night_phase::{DayNightPhase, DayNightPhases},
};

#[derive(Clone, Copy, Deserialize)]
pub struct DayNightLightingPhase {
    pub start_time: f32,
    pub ambient_color: Rgb,
    pub ambient_brightness: f32,
    pub sun_color: Rgb,
    pub sun_illuminance: f32,
}

#[derive(Clone, Copy)]
pub struct DayNightSample {
    pub phase: DayNightPhase,
    pub next_phase: DayNightPhase,
    pub transition: f32,
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
    pub phases: DayNightPhases<DayNightLightingPhase>,
}

impl DayNightCycleDefinition {
    pub fn sample(&self, time: f32) -> DayNightSample {
        let time = time.rem_euclid(1.0);
        let phase = self.phase_at(time);
        let next_phase = DayNightPhases::<DayNightLightingPhase>::next(phase);
        let current = self.phases.get(phase);
        let next = self.phases.get(next_phase);

        let current_start = current.start_time;
        let next_start = if next.start_time <= current_start {
            next.start_time + 1.0
        } else {
            next.start_time
        };
        let sample_time = if time < current_start { time + 1.0 } else { time };
        let span = next_start - current_start;
        let transition = if span <= f32::EPSILON {
            0.0
        } else {
            ((sample_time - current_start) / span).clamp(0.0, 1.0)
        };

        DayNightSample {
            phase,
            next_phase,
            transition,
            ambient_color: current.ambient_color.lerp(next.ambient_color, transition),
            ambient_brightness: lerp_scalar(
                current.ambient_brightness,
                next.ambient_brightness,
                transition,
            ),
            sun_color: current.sun_color.lerp(next.sun_color, transition),
            sun_illuminance: lerp_scalar(
                current.sun_illuminance,
                next.sun_illuminance,
                transition,
            ),
        }
    }

    pub fn phase_at(&self, time: f32) -> DayNightPhase {
        let time = time.rem_euclid(1.0);
        let dawn = self.phases.dawn.start_time;
        let day = self.phases.day.start_time;
        let dusk = self.phases.dusk.start_time;
        let night = self.phases.night.start_time;

        if time >= night || time < dawn {
            DayNightPhase::Night
        } else if time >= dusk {
            DayNightPhase::Dusk
        } else if time >= day {
            DayNightPhase::Day
        } else {
            DayNightPhase::Dawn
        }
    }
}

#[derive(Resource, Default)]
pub struct DayNightCycleRegistry {
    definitions: HashMap<String, DayNightCycleDefinition>,
}

impl DayNightCycleRegistry {
    pub fn insert(&mut self, definition: DayNightCycleDefinition) {
        assert!(
            definition.phases.dawn.start_time < definition.phases.day.start_time
                && definition.phases.day.start_time < definition.phases.dusk.start_time
                && definition.phases.dusk.start_time < definition.phases.night.start_time,
            "day-night cycle {} phase start times must be ordered dawn < day < dusk < night",
            definition.id
        );

        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&DayNightCycleDefinition> {
        self.definitions.get(id)
    }
}

fn lerp_scalar(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}
