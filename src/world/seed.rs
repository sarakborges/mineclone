use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;

use super::deterministic::mix_seed;

#[derive(Resource, Clone, Copy, Debug)]
pub struct WorldSeed(pub u64);

impl WorldSeed {
    pub fn fresh() -> Self {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let low = elapsed as u64;
        let high = (elapsed >> 64) as u64;

        Self(mix_seed(low ^ high.rotate_left(17)))
    }
}

impl Default for WorldSeed {
    fn default() -> Self {
        Self::fresh()
    }
}
