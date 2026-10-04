use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;

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

fn mix_seed(mut seed: u64) -> u64 {
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xff51_afd7_ed55_8ccd);
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    seed ^= seed >> 33;
    seed
}
