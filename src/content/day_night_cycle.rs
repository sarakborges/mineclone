use std::collections::HashSet;

use bevy::prelude::*;
use serde::Deserialize;

use super::{
    day_night_phase::{DayNightPhase, DayNightPhases},
    registry::DefinitionMap,
};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayNightPhaseTiming {
    pub duration_ticks: u64,
    pub sky_light_factor: f32,
}

#[derive(Clone, Copy)]
pub struct DayNightSample {
    pub phase: DayNightPhase,
    pub next_phase: DayNightPhase,
    pub transition: f32,
    pub sky_light_factor: f32,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayNightCycleDefinition {
    pub id: String,
    pub day_duration_ticks: u64,
    pub initial_time: f32,
    pub world_time_start_hour: f32,
    pub sequence: [DayNightPhase; 4],
    pub phases: DayNightPhases<DayNightPhaseTiming>,
}

impl DayNightCycleDefinition {
    pub fn sample(&self, time: f32) -> DayNightSample {
        let elapsed_ticks = time.rem_euclid(1.0) * self.day_duration_ticks as f32;
        let (phase, phase_elapsed_ticks) = self.phase_at_elapsed(elapsed_ticks);
        let next_phase = self.next_phase(phase);
        let current = self.phases.get(phase);
        let next = self.phases.get(next_phase);
        let transition = if current.duration_ticks == 0 {
            0.0
        } else {
            (phase_elapsed_ticks / current.duration_ticks as f32).clamp(0.0, 1.0)
        };

        DayNightSample {
            phase,
            next_phase,
            transition,
            sky_light_factor: lerp_scalar(
                current.sky_light_factor,
                next.sky_light_factor,
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
        let start_ticks = self.phase_start_ticks(start_phase)? as f32;
        let mut end_ticks = self.phase_start_ticks(end_phase)? as f32
            + self.phases.get(end_phase).duration_ticks as f32;
        let mut current_ticks = normalized_time.rem_euclid(1.0) * self.day_duration_ticks as f32;

        if end_ticks <= start_ticks {
            end_ticks += self.day_duration_ticks as f32;
        }
        if current_ticks < start_ticks {
            current_ticks += self.day_duration_ticks as f32;
        }
        if current_ticks < start_ticks || current_ticks > end_ticks {
            return None;
        }

        Some(((current_ticks - start_ticks) / (end_ticks - start_ticks)).clamp(0.0, 1.0))
    }

    fn phase_at_elapsed(&self, elapsed_ticks: f32) -> (DayNightPhase, f32) {
        let mut cursor = 0.0;

        for phase in self.sequence {
            let duration = self.phases.get(phase).duration_ticks as f32;
            let end = cursor + duration;

            if elapsed_ticks < end {
                return (phase, elapsed_ticks - cursor);
            }

            cursor = end;
        }

        let phase = self.sequence[3];
        (phase, self.phases.get(phase).duration_ticks as f32)
    }

    fn next_phase(&self, phase: DayNightPhase) -> DayNightPhase {
        let index = self
            .sequence
            .iter()
            .position(|candidate| *candidate == phase)
            .expect("validated day-night sequence must contain every phase");

        self.sequence[(index + 1) % self.sequence.len()]
    }

    fn phase_start_ticks(&self, target: DayNightPhase) -> Option<u64> {
        let mut cursor = 0;

        for phase in self.sequence {
            if phase == target {
                return Some(cursor);
            }

            cursor += self.phases.get(phase).duration_ticks;
        }

        None
    }
}

#[derive(Resource, Default)]
pub struct DayNightCycleRegistry {
    definitions: DefinitionMap<DayNightCycleDefinition>,
}

impl DayNightCycleRegistry {
    pub fn insert(&mut self, definition: DayNightCycleDefinition) {
        assert!(
            definition.day_duration_ticks > 0,
            "day-night cycle {} day duration must be positive",
            definition.id
        );

        let phase_duration = definition.phases.dawn.duration_ticks
            + definition.phases.day.duration_ticks
            + definition.phases.dusk.duration_ticks
            + definition.phases.night.duration_ticks;

        assert_eq!(
            phase_duration,
            definition.day_duration_ticks,
            "day-night cycle {} phase durations ({phase_duration}) must equal day duration ({})",
            definition.id,
            definition.day_duration_ticks
        );

        for phase in [
            definition.phases.dawn,
            definition.phases.day,
            definition.phases.dusk,
            definition.phases.night,
        ] {
            assert!(
                (0.0..=1.0).contains(&phase.sky_light_factor),
                "day-night cycle {} sky light factors must be between 0 and 1",
                definition.id
            );
        }

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
