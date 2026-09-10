use crate::content::fluid::FluidId;

pub const MAX_FLUID_LEVEL: u8 = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FluidCell {
    pub fluid_id: FluidId,
    pub level: u8,
}

impl FluidCell {
    pub fn new(fluid_id: FluidId, level: u8) -> Self {
        assert!(
            (1..=MAX_FLUID_LEVEL).contains(&level),
            "fluid level must be between 1 and {MAX_FLUID_LEVEL}"
        );

        Self { fluid_id, level }
    }

    pub fn height(self) -> f32 {
        self.level as f32 / MAX_FLUID_LEVEL as f32
    }
}
