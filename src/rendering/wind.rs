use bevy::prelude::*;

#[derive(Resource, Clone, Copy, Debug)]
pub(crate) struct Wind {
    pub(crate) direction: Vec2,
    pub(crate) strength: f32,
}

impl Default for Wind {
    fn default() -> Self {
        Self {
            direction: Vec2::new(0.93, 0.37).normalize(),
            strength: 0.45,
        }
    }
}

impl Wind {
    pub(crate) fn velocity(self, influence: f32) -> Vec3 {
        let direction = self.direction.normalize_or_zero();
        Vec3::new(direction.x, 0.0, direction.y) * self.strength * influence
    }
}
