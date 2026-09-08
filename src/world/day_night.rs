use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{day_night_cycle::DayNightCycleRegistry, dimension::DimensionRegistry},
};

use super::dimension::CurrentDimension;

#[derive(Resource)]
pub struct DayNightClock {
    pub day: u64,
    pub normalized_time: f32,
}

impl Default for DayNightClock {
    fn default() -> Self {
        Self {
            day: 1,
            normalized_time: 0.0,
        }
    }
}

pub struct DayNightPlugin;

impl Plugin for DayNightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DayNightClock>()
            .add_systems(OnEnter(GameState::Gameplay), initialize_clock)
            .add_systems(
                PreUpdate,
                advance_clock.run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn initialize_clock(
    dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    cycles: Res<DayNightCycleRegistry>,
    mut clock: ResMut<DayNightClock>,
) {
    let dimension = dimensions
        .get(&dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", dimension.id));
    let cycle = cycles
        .get(&dimension.day_night_cycle)
        .unwrap_or_else(|| panic!("missing day-night cycle: {}", dimension.day_night_cycle));

    clock.day = 1;
    clock.normalized_time = cycle.initial_time.rem_euclid(1.0);
}

fn advance_clock(
    time: Res<Time>,
    dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    cycles: Res<DayNightCycleRegistry>,
    mut clock: ResMut<DayNightClock>,
) {
    let Some(dimension) = dimensions.get(&dimension.id) else {
        return;
    };
    let Some(cycle) = cycles.get(&dimension.day_night_cycle) else {
        return;
    };

    if cycle.day_duration_seconds <= 0.0 {
        return;
    }

    let elapsed_days = time.delta_secs() / cycle.day_duration_seconds;
    let advanced_time = clock.normalized_time + elapsed_days;
    let completed_days = advanced_time.floor().max(0.0) as u64;

    clock.day += completed_days;
    clock.normalized_time = advanced_time.rem_euclid(1.0);
}
