use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::*;

const DEBUG_WORLD_SEED_ENV: &str = "MINECLONE_DEBUG_WORLD_SEED";

#[derive(Resource, Clone, Copy, Debug)]
pub struct WorldSeed(pub u64);

impl WorldSeed {
    pub fn fresh() -> Self {
        if let Some(seed) = debug_seed_from_environment() {
            return Self(seed);
        }

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

fn debug_seed_from_environment() -> Option<u64> {
    let value = env::var(DEBUG_WORLD_SEED_ENV).ok()?;

    Some(parse_debug_seed(&value).unwrap_or_else(|| {
        panic!(
            "{DEBUG_WORLD_SEED_ENV} must be an unsigned 64-bit integer, got: {value}"
        )
    }))
}

fn parse_debug_seed(value: &str) -> Option<u64> {
    value.trim().parse().ok()
}

fn mix_seed(mut seed: u64) -> u64 {
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xff51_afd7_ed55_8ccd);
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    seed ^= seed >> 33;
    seed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_seed_parser_accepts_reproducible_u64_values() {
        assert_eq!(parse_debug_seed("12345"), Some(12345));
        assert_eq!(parse_debug_seed(" 42 "), Some(42));
        assert_eq!(parse_debug_seed("not-a-seed"), None);
    }
}
