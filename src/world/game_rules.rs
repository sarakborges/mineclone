use bevy::prelude::*;

pub(crate) const DEFAULT_TICKS_PER_SECOND: u32 = 40;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GameRules {
    ticks_per_second: u32,
}

impl Default for GameRules {
    fn default() -> Self {
        Self {
            ticks_per_second: DEFAULT_TICKS_PER_SECOND,
        }
    }
}

impl GameRules {
    pub(crate) fn ticks_per_second(&self) -> u32 {
        self.ticks_per_second
    }

    pub(crate) fn tick_seconds(&self) -> f32 {
        1.0 / self.ticks_per_second as f32
    }

    pub(crate) fn set_ticks_per_second(&mut self, ticks_per_second: u32) {
        assert!(ticks_per_second > 0, "ticks per second must be greater than zero");
        self.ticks_per_second = ticks_per_second;
    }
}
