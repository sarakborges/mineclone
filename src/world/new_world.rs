use bevy::prelude::*;

use crate::player::game_mode::GameMode;

use super::{WorldSeed, game_rules::GameRules, world_names::DEFAULT_WORLD_NAME};

/// Compatibility identity for deterministic terrain generation.
///
/// Increment this whenever an algorithm change can make an untouched chunk
/// generate differently for the same world seed/configuration. Saved worlds
/// pin this value in their immutable generation-zero manifest so a newer game
/// never silently mixes two world-generation algorithms in one world.
pub(crate) const WORLDGEN_VERSION: u32 = 1;

/// Saves created before the worldgen identity field existed used the same
/// generator now identified as version 1. Keep this fallback fixed forever:
/// defaulting legacy saves to `WORLDGEN_VERSION` would silently reinterpret
/// them after a future generator bump.
pub(crate) const LEGACY_WORLDGEN_VERSION: u32 = 1;

const MIN_BIOME_SIZE_MULTIPLIER_TENTHS: u8 = 5;
const MAX_BIOME_SIZE_MULTIPLIER_TENTHS: u8 = 50;
const DEFAULT_BIOME_SIZE_MULTIPLIER_TENTHS: u8 = 10;

pub(crate) const MIN_BIOME_SIZE_MULTIPLIER: f32 =
    MIN_BIOME_SIZE_MULTIPLIER_TENTHS as f32 / 10.0;
pub(crate) const MAX_BIOME_SIZE_MULTIPLIER: f32 =
    MAX_BIOME_SIZE_MULTIPLIER_TENTHS as f32 / 10.0;
pub(crate) const DEFAULT_BIOME_SIZE_MULTIPLIER: f32 =
    DEFAULT_BIOME_SIZE_MULTIPLIER_TENTHS as f32 / 10.0;

#[derive(Resource, Debug, Clone)]
pub(crate) struct NewWorldConfig {
    name: String,
    seed: WorldSeed,
    game_mode: GameMode,
    game_rules: GameRules,
    spawn_biome: Option<String>,
    biome_size_multiplier_tenths: u8,
}

impl Default for NewWorldConfig {
    fn default() -> Self {
        Self {
            name: DEFAULT_WORLD_NAME.to_owned(),
            seed: WorldSeed::default(),
            game_mode: GameMode::default(),
            game_rules: GameRules::default(),
            spawn_biome: None,
            biome_size_multiplier_tenths: DEFAULT_BIOME_SIZE_MULTIPLIER_TENTHS,
        }
    }
}

impl NewWorldConfig {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn set_name(&mut self, name: String) {
        if self.name != name {
            self.name = name;
        }
    }

    pub(crate) fn seed(&self) -> WorldSeed {
        self.seed
    }

    pub(crate) fn set_seed(&mut self, seed: u64) {
        self.seed = WorldSeed(seed);
    }

    pub(crate) fn game_mode(&self) -> GameMode {
        self.game_mode
    }

    pub(crate) fn set_game_mode(&mut self, game_mode: GameMode) {
        self.game_mode = game_mode;
    }

    pub(crate) fn game_rules(&self) -> GameRules {
        self.game_rules
    }

    pub(crate) fn set_ticks_per_second(&mut self, ticks_per_second: u32) {
        self.game_rules.set_ticks_per_second(ticks_per_second);
    }

    pub(crate) fn spawn_biome(&self) -> Option<&str> {
        self.spawn_biome.as_deref()
    }

    pub(crate) fn set_spawn_biome(&mut self, spawn_biome: Option<String>) {
        if self.spawn_biome != spawn_biome {
            self.spawn_biome = spawn_biome;
        }
    }

    pub(crate) fn biome_size_multiplier(&self) -> f32 {
        self.biome_size_multiplier_tenths as f32 / 10.0
    }

    pub(crate) fn set_biome_size_multiplier(&mut self, value: f32) {
        self.biome_size_multiplier_tenths =
            biome_size_multiplier_tenths(snap_biome_size_multiplier(value))
                .expect("snapped biome size multiplier must be valid");
    }
}

pub(crate) fn snap_biome_size_multiplier(value: f32) -> f32 {
    if !value.is_finite() {
        return DEFAULT_BIOME_SIZE_MULTIPLIER;
    }

    let tenths = (value * 10.0)
        .round()
        .clamp(
            MIN_BIOME_SIZE_MULTIPLIER_TENTHS as f32,
            MAX_BIOME_SIZE_MULTIPLIER_TENTHS as f32,
        ) as u8;
    tenths as f32 / 10.0
}

pub(crate) fn biome_size_multiplier_tenths(value: f32) -> Option<u8> {
    if !value.is_finite() {
        return None;
    }

    let scaled = value * 10.0;
    let rounded = scaled.round();
    if (scaled - rounded).abs() > 0.0001 {
        return None;
    }

    let tenths = rounded as i32;
    (i32::from(MIN_BIOME_SIZE_MULTIPLIER_TENTHS)
        ..=i32::from(MAX_BIOME_SIZE_MULTIPLIER_TENTHS))
        .contains(&tenths)
        .then_some(tenths as u8)
}

pub(crate) fn is_valid_biome_size_multiplier(value: f32) -> bool {
    biome_size_multiplier_tenths(value).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biome_size_multiplier_defaults_to_one() {
        assert_eq!(
            NewWorldConfig::default().biome_size_multiplier(),
            DEFAULT_BIOME_SIZE_MULTIPLIER
        );
    }

    #[test]
    fn biome_size_multiplier_snaps_and_clamps_to_tenths() {
        assert_eq!(snap_biome_size_multiplier(0.1), 0.5);
        assert_eq!(snap_biome_size_multiplier(1.04), 1.0);
        assert_eq!(snap_biome_size_multiplier(1.06), 1.1);
        assert_eq!(snap_biome_size_multiplier(6.0), 5.0);
        assert!(is_valid_biome_size_multiplier(2.3));
        assert!(!is_valid_biome_size_multiplier(2.34));
    }

    #[test]
    fn legacy_worldgen_identity_stays_pinned_to_v1() {
        assert_eq!(LEGACY_WORLDGEN_VERSION, 1);
        assert!(WORLDGEN_VERSION >= LEGACY_WORLDGEN_VERSION);
    }
}
