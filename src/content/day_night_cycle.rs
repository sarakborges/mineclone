use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use serde::Deserialize;

use super::{
    color::Rgb,
    day_night_phase::{DayNightPhase, DayNightPhases},
};

#[derive(Clone, Copy, Deserialize)]
pub struct DayNightLightingPhase {
    pub duration_seconds: f32,
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
    pub day_duration_seconds: f32,
    pub initial_time: f32,
    pub world_time_start_hour: f32,
    pub sun_angle_offset_degrees: f32,
    pub sequence: [DayNightPhase; 4],
    pub phases: DayNightPhases<DayNightLightingPhase>,
}

impl DayNightCycleDefinition {
    pub fn sample(&self, time: f32) -> DayNightSample {
        let elapsed_seconds = time.rem_euclid(1.0) * self.day_duration_seconds;
        let (phase, phase_elapsed_seconds) = self.phase_at_elapsed(elapsed_seconds);
        let next_phase = self.next_phase(phase);
        let current = self.phases.get(phase);
        let next = self.phases.get(next_phase);
        let transition = if current.duration_seconds <= f32::EPSILON {
            0.0
        } else {
            (phase_elapsed_seconds / current.duration_seconds).clamp(0.0, 1.0)
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

    pub fn world_time(&self, normalized_time: f32) -> (u32, u32) {
        let start_minutes = self.world_time_start_hour.rem_euclid(24.0) * 60.0;
        let elapsed_minutes = normalized_time.rem_euclid(1.0) * 24.0 * 60.0;
        let total_minutes = (start_minutes + elapsed_minutes).rem_euclid(24.0 * 60.0);
        let rounded_minutes = total_minutes.floor() as u32;

        (rounded_minutes / 60, rounded_minutes % 60)
    }

    pub fn progress_between_phases(
        &self,
        normalized_time: f32,
        start_phase: DayNightPhase,
        end_phase: DayNightPhase,
    ) -> Option<f32> {
        let start_seconds = self.phase_start_seconds(start_phase)?;
        let mut end_seconds = self.phase_start_seconds(end_phase)?
            + self.phases.get(end_phase).duration_seconds;
        let mut current_seconds = normalized_time.rem_euclid(1.0) * self.day_duration_seconds;

        if end_seconds <= start_seconds {
            end_seconds += self.day_duration_seconds;
        }
        if current_seconds < start_seconds {
            current_seconds += self.day_duration_seconds;
        }
        if current_seconds < start_seconds || current_seconds > end_seconds {
            return None;
        }

        Some(((current_seconds - start_seconds) / (end_seconds - start_seconds)).clamp(0.0, 1.0))
    }

    fn phase_at_elapsed(&self, elapsed_seconds: f32) -> (DayNightPhase, f32) {
        let mut cursor = 0.0;

        for phase in self.sequence {
            let duration = self.phases.get(phase).duration_seconds;
            let end = cursor + duration;

            if elapsed_seconds < end {
                return (phase, elapsed_seconds - cursor);
            }

            cursor = end;
        }

        let phase = self.sequence[3];
        (phase, self.phases.get(phase).duration_seconds)
    }

    fn next_phase(&self, phase: DayNightPhase) -> DayNightPhase {
        let index = self
            .sequence
            .iter()
            .position(|candidate| *candidate == phase)
            .expect("validated day-night sequence must contain every phase");

        self.sequence[(index + 1) % self.sequence.len()]
    }

    fn phase_start_seconds(&self, target: DayNightPhase) -> Option<f32> {
        let mut cursor = 0.0;

        for phase in self.sequence {
            if phase == target {
                return Some(cursor);
            }

            cursor += self.phases.get(phase).duration_seconds;
        }

        None
    }
}

#[derive(Resource, Default)]
pub struct DayNightCycleRegistry {
    definitions: HashMap<String, DayNightCycleDefinition>,
}

impl DayNightCycleRegistry {
    pub fn insert(&mut self, definition: DayNightCycleDefinition) {
        assert!(
            definition.day_duration_seconds > 0.0,
            "day-night cycle {} day duration must be positive",
            definition.id
        );

        let phase_duration = definition.phases.dawn.duration_seconds
            + definition.phases.day.duration_seconds
            + definition.phases.dusk.duration_seconds
            + definition.phases.night.duration_seconds;

        assert!(
            (phase_duration - definition.day_duration_seconds).abs() <= 0.001,
            "day-night cycle {} phase durations ({phase_duration}) must equal day duration ({})",
            definition.id,
            definition.day_duration_seconds
        );

        let unique_phases = definition.sequence.iter().copied().collect::<HashSet<_>>();
        assert!(
            unique_phases.len() == 4,
            "day-night cycle {} sequence must contain Dawn, Day, Dusk and Night exactly once",
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
