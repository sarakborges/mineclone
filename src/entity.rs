use bevy::prelude::*;

/// Shared gameplay health for every damageable entity, including the player.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct EntityHealth {
    current: f32,
    max: f32,
}

impl EntityHealth {
    pub(crate) fn new(max: f32) -> Self {
        assert!(max.is_finite() && max > 0.0, "entity health must be positive and finite");
        Self { current: max, max }
    }

    pub(crate) fn restored(max: f32, current: f32) -> Self {
        assert!(max.is_finite() && max > 0.0, "entity health must be positive and finite");
        assert!(
            current.is_finite() && current >= 0.0,
            "restored entity health must be finite and non-negative"
        );
        Self {
            current: current.min(max),
            max,
        }
    }

    pub(crate) fn current(self) -> f32 { self.current }
    pub(crate) fn max(self) -> f32 { self.max }
    pub(crate) fn is_dead(self) -> bool { self.current <= 0.0 }

    pub(crate) fn damage(&mut self, amount: f32) -> bool {
        if self.is_dead() { return true; }
        self.current = (self.current - amount.max(0.0)).max(0.0);
        self.is_dead()
    }

}

