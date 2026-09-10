use bevy::prelude::*;

#[derive(Clone, Debug, Default)]
pub struct GeologyRegion;

impl GeologyRegion {
    pub fn density_delta(&self, _position: Vec3) -> f32 {
        0.0
    }

    pub fn solid_block_at(&self, _position: Vec3) -> Option<&str> {
        None
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GeologyField;

impl GeologyField {
    pub fn new(_seed: u64) -> Self {
        Self
    }

    pub fn region(&self, _coord: IVec3) -> GeologyRegion {
        GeologyRegion
    }
}
