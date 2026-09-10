use crate::content::fluid::FluidId;

pub const MAX_FLUID_LEVEL: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluidCell {
    pub fluid_id: FluidId,
    pub level: u8,
    source: bool,
    spread_distance: u16,
}

impl FluidCell {
    pub fn new(fluid_id: FluidId, level: u8) -> Self {
        Self::source(fluid_id, level)
    }

    pub fn source(fluid_id: FluidId, level: u8) -> Self {
        Self::with_state(fluid_id, level, true, 0)
    }

    pub fn flowing(fluid_id: FluidId, level: u8) -> Self {
        Self::with_state(fluid_id, level, false, 0)
    }

    pub fn spreading(fluid_id: FluidId, level: u8, spread_distance: u16) -> Self {
        Self::with_state(fluid_id, level, false, spread_distance)
    }

    pub fn with_source(fluid_id: FluidId, level: u8, source: bool) -> Self {
        Self::with_state(fluid_id, level, source, 0)
    }

    pub fn with_state(fluid_id: FluidId, level: u8, source: bool, spread_distance: u16) -> Self {
        assert!(
            (1..=MAX_FLUID_LEVEL).contains(&level),
            "fluid level must be between 1 and {MAX_FLUID_LEVEL}"
        );

        Self {
            fluid_id,
            level,
            source,
            spread_distance,
        }
    }

    pub fn is_source(self) -> bool {
        self.source
    }

    pub fn spread_distance(self) -> u16 {
        self.spread_distance
    }

    pub fn height(self) -> f32 {
        self.level as f32 / MAX_FLUID_LEVEL as f32
    }
}
