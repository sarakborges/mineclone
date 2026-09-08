use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::color::Rgb;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum DayNightPhase {
    Dawn,
    Day,
    Dusk,
    Night,
}

#[derive(Clone, Copy, Deserialize)]
pub struct DayNightPhaseDefinition {
    pub phase: DayNightPhase,
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
    pub phases: Vec<DayNightPhaseDefinition>,
}

impl DayNightCycleDefinition {
    pub fn sample(&self, time: f32) -> Option<DayNightSample> {
        if self.phases.len() != 4 {
            return None;
        }

        let time = time.rem_euclid(1.0);
        let current_index = self
            .phases
            .iter()
            .rposition(|phase| phase.start_time <= time)
            .unwrap_or(self.phases.len() - 1);
        let next_index = (current_index + 1) % self.phases.len();
        let current = self.phases[current_index];
        let next = self.phases[next_index];

        let current_start = current.start_time;
        let next_start = if next_index == 0 {
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

        Some(DayNightSample {
            phase: current.phase,
            next_phase: next.phase,
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
        })
    }
}

#[derive(Resource, Default)]
pub struct DayNightCycleRegistry {
    definitions: HashMap<String, DayNightCycleDefinition>,
}

impl DayNightCycleRegistry {
    pub fn insert(&mut self, definition: DayNightCycleDefinition) {
        assert_eq!(
            definition.phases.len(),
            4,
            "day-night cycle {} must define exactly four phases",
            definition.id
        );

        let expected = [
            DayNightPhase::Dawn,
            DayNightPhase::Day,
            DayNightPhase::Dusk,
            DayNightPhase::Night,
        ];

        for (phase, expected_phase) in definition.phases.iter().zip(expected) {
            assert_eq!(
                phase.phase, expected_phase,
                "day-night cycle {} phases must be ordered Dawn, Day, Dusk, Night",
                definition.id
            );
        }

        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&DayNightCycleDefinition> {
        self.definitions.get(id)
    }
}

fn lerp_scalar(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}
