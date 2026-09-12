use bevy::prelude::*;

use super::game_rules::GameRules;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct WorldTickSet;

#[derive(Resource, Default)]
pub(crate) struct WorldTickClock {
    tick: u64,
    accumulated_ticks: f64,
    ticks_this_frame: u32,
}

impl WorldTickClock {
    pub(crate) fn current_tick(&self) -> u64 {
        self.tick
    }

    pub(crate) fn ticks_this_frame(&self) -> u32 {
        self.ticks_this_frame
    }

    pub(crate) fn delta_seconds(&self, game_rules: &GameRules) -> f32 {
        self.ticks_this_frame as f32 * game_rules.tick_seconds()
    }
}

pub(crate) fn reset_world_ticks(mut clock: ResMut<WorldTickClock>) {
    *clock = WorldTickClock::default();
}

pub(crate) fn advance_world_ticks(
    time: Res<Time<Real>>,
    game_rules: Res<GameRules>,
    mut clock: ResMut<WorldTickClock>,
) {
    clock.ticks_this_frame = 0;
    clock.accumulated_ticks +=
        time.delta().as_secs_f64() * game_rules.ticks_per_second() as f64;

    let elapsed_ticks = clock.accumulated_ticks.floor().min(u32::MAX as f64) as u32;
    if elapsed_ticks == 0 {
        return;
    }

    clock.accumulated_ticks -= elapsed_ticks as f64;
    clock.tick = clock.tick.saturating_add(elapsed_ticks as u64);
    clock.ticks_this_frame = elapsed_ticks;
}
