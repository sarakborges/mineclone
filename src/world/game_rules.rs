use bevy::prelude::*;

pub(crate) const DEFAULT_TICKS_PER_SECOND: u32 = 40;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GameRules {
    ticks_per_second: u32,
    spawn_creatures: bool,
    spawn_caves: bool,
    spawn_rivers: bool,
    spawn_lakes: bool,
    spawn_oceans: bool,
}

impl Default for GameRules {
    fn default() -> Self {
        Self {
            ticks_per_second: DEFAULT_TICKS_PER_SECOND,
            spawn_creatures: true,
            spawn_caves: true,
            spawn_rivers: true,
            spawn_lakes: true,
            spawn_oceans: true,
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

    pub(crate) fn spawn_creatures(&self) -> bool {
        self.spawn_creatures
    }

    pub(crate) fn set_spawn_creatures(&mut self, spawn_creatures: bool) {
        self.spawn_creatures = spawn_creatures;
    }

    pub(crate) const fn spawn_caves(&self) -> bool {
        self.spawn_caves
    }

    pub(crate) fn set_spawn_caves(&mut self, spawn_caves: bool) {
        self.spawn_caves = spawn_caves;
    }

    pub(crate) const fn spawn_rivers(&self) -> bool {
        self.spawn_rivers
    }

    pub(crate) fn set_spawn_rivers(&mut self, spawn_rivers: bool) {
        self.spawn_rivers = spawn_rivers;
    }

    pub(crate) const fn spawn_lakes(&self) -> bool {
        self.spawn_lakes
    }

    pub(crate) fn set_spawn_lakes(&mut self, spawn_lakes: bool) {
        self.spawn_lakes = spawn_lakes;
    }

    pub(crate) const fn spawn_oceans(&self) -> bool {
        self.spawn_oceans
    }

    pub(crate) fn set_spawn_oceans(&mut self, spawn_oceans: bool) {
        self.spawn_oceans = spawn_oceans;
    }

    pub(crate) fn set_worldgen_hydrology(
        &mut self,
        spawn_caves: bool,
        spawn_rivers: bool,
        spawn_lakes: bool,
        spawn_oceans: bool,
    ) {
        self.spawn_caves = spawn_caves;
        self.spawn_rivers = spawn_rivers;
        self.spawn_lakes = spawn_lakes;
        self.spawn_oceans = spawn_oceans;
    }

    pub(crate) fn set_ticks_per_second(&mut self, ticks_per_second: u32) {
        assert!(ticks_per_second > 0, "ticks per second must be greater than zero");
        self.ticks_per_second = ticks_per_second;
    }
}
