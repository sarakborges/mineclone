use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{day_night_cycle::DayNightCycleRegistry, dimension::DimensionRegistry},
};

use super::{
    WorldLoadMode,
    dimension::CurrentDimension,
    tick::{WorldTickClock, WorldTickSet},
};

#[derive(Resource)]
pub struct DayNightClock {
    pub day: u64,
    pub normalized_time: f32,
    tick_in_day: u64,
}

impl Default for DayNightClock {
    fn default() -> Self {
        Self {
            day: 1,
            normalized_time: 0.0,
            tick_in_day: 0,
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
                advance_clock
                    .after(WorldTickSet)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn initialize_clock(
    dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    cycles: Res<DayNightCycleRegistry>,
    load_mode: Res<WorldLoadMode>,
    mut clock: ResMut<DayNightClock>,
) {
    if *load_mode == WorldLoadMode::Load {
        return;
    }

    let dimension = dimensions
        .get(&dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", dimension.id));
    let cycle = cycles
        .get(&dimension.day_night_cycle)
        .unwrap_or_else(|| panic!("missing day-night cycle: {}", dimension.day_night_cycle));

    clock.day = 1;
    clock.tick_in_day =
        (cycle.initial_time.rem_euclid(1.0) * cycle.day_duration_ticks as f32).floor() as u64;
    clock.normalized_time =
        clock.tick_in_day as f32 / cycle.day_duration_ticks.max(1) as f32;
}

fn advance_clock(
    dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    cycles: Res<DayNightCycleRegistry>,
    world_ticks: Res<WorldTickClock>,
    mut clock: ResMut<DayNightClock>,
) {
    let elapsed_ticks = world_ticks.ticks_this_frame() as u64;
    if elapsed_ticks == 0 {
        return;
    }

    let Some(dimension) = dimensions.get(&dimension.id) else {
        return;
    };
    let Some(cycle) = cycles.get(&dimension.day_night_cycle) else {
        return;
    };

    let day_duration_ticks = cycle.day_duration_ticks;
    if day_duration_ticks == 0 {
        return;
    }

    let advanced_ticks = clock.tick_in_day.saturating_add(elapsed_ticks);
    clock.day = clock
        .day
        .saturating_add(advanced_ticks / day_duration_ticks);
    clock.tick_in_day = advanced_ticks % day_duration_ticks;
    clock.normalized_time = clock.tick_in_day as f32 / day_duration_ticks as f32;
}
