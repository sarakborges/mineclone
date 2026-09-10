use crate::content::fluid::FluidId;

pub const MAX_FLUID_LEVEL: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluidCell {
    pub fluid_id: FluidId,
    pub level: u8,
    source: bool,
}

impl FluidCell {
    pub fn new(fluid_id: FluidId, level: u8) -> Self {
        Self::source(fluid_id, level)
    }

    pub fn source(fluid_id: FluidId, level: u8) -> Self {
        Self::with_source(fluid_id, level, true)
    }

    pub fn flowing(fluid_id: FluidId, level: u8) -> Self {
        Self::with_source(fluid_id, level, false)
    }

    pub fn with_source(fluid_id: FluidId, level: u8, source: bool) -> Self {
        assert!(
            (1..=MAX_FLUID_LEVEL).contains(&level),
            "fluid level must be between 1 and {MAX_FLUID_LEVEL}"
        );

        Self {
            fluid_id,
            level,
            source,
        }
    }

    pub fn is_source(self) -> bool {
        self.source
    }

    pub fn height(self) -> f32 {
        self.level as f32 / MAX_FLUID_LEVEL as f32
    }
}
