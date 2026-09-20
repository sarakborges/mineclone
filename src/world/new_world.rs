use std::io;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::player::game_mode::GameMode;

use super::{WorldSeed, game_rules::GameRules, world_names::DEFAULT_WORLD_NAME};

/// Compatibility identity for deterministic terrain generation.
///
/// Increment this whenever an algorithm change can make an untouched chunk
/// generate differently for the same world seed/configuration. Saved worlds
/// pin this value in their immutable generation-zero manifest so a newer game
/// never silently mixes two world-generation algorithms in one world.
pub(crate) const WORLDGEN_VERSION: u32 = 1;

/// Persisted deterministic-generator identity. The transparent representation
/// keeps manifests/snapshots human-readable while making it harder for save
/// boundaries to accidentally confuse this compatibility identity with an
/// unrelated format/generation counter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct WorldgenVersion(u32);

impl WorldgenVersion {
    pub(crate) const fn current() -> Self {
        Self(WORLDGEN_VERSION)
    }

    pub(crate) fn validate(self) -> io::Result<()> {
        validate_worldgen_version(self.0)
    }

    pub(crate) fn validate_matches(self, snapshot: Self) -> io::Result<()> {
        validate_matching_worldgen_versions(self.0, snapshot.0)
    }
}

/// A save may only generate untouched terrain when it was created with the
/// exact deterministic generator implemented by this build. Keep this strict:
/// accepting older/newer identities would silently mix terrain algorithms.
pub(crate) fn is_compatible_worldgen_version(version: u32) -> bool {
    version == WORLDGEN_VERSION
}

/// Shared validation used by persistence boundaries before a save is allowed
/// to regenerate untouched terrain. Return the persistence layer's native
/// error type so every boundary can propagate the same diagnostic unchanged.
pub(crate) fn validate_worldgen_version(version: u32) -> io::Result<()> {
    if is_compatible_worldgen_version(version) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "saved world uses an incompatible world-generation version",
        ))
    }
}

/// A snapshot must agree with the world's immutable generation-zero identity,
/// and that identity must be supported by this build before untouched chunks
/// may be regenerated. Keep both checks here so save/load boundaries cannot
/// accidentally validate only one side of the invariant.
pub(crate) fn validate_matching_worldgen_versions(
    reserved_version: u32,
    snapshot_version: u32,
) -> io::Result<()> {
    validate_worldgen_version(reserved_version)?;
    if snapshot_version != reserved_version {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "snapshot does not match reserved world-generation identity",
        ));
    }
    Ok(())
}

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
    fn worldgen_compatibility_requires_exact_current_identity() {
        assert!(is_compatible_worldgen_version(WORLDGEN_VERSION));
        assert!(!is_compatible_worldgen_version(0));
        assert!(!is_compatible_worldgen_version(
            WORLDGEN_VERSION.saturating_add(1)
        ));
        assert!(validate_worldgen_version(WORLDGEN_VERSION).is_ok());
        assert!(WorldgenVersion::current().validate().is_ok());
        let error = validate_worldgen_version(WORLDGEN_VERSION.saturating_add(1))
            .expect_err("future worldgen identity must be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            error.to_string(),
            "saved world uses an incompatible world-generation version"
        );
    }

    #[test]
    fn persisted_worldgen_identities_must_match_each_other_and_this_build() {
        assert!(validate_matching_worldgen_versions(WORLDGEN_VERSION, WORLDGEN_VERSION).is_ok());
        assert!(WorldgenVersion::current()
            .validate_matches(WorldgenVersion::current())
            .is_ok());

        let mismatched = validate_matching_worldgen_versions(
            WORLDGEN_VERSION,
            WORLDGEN_VERSION.saturating_add(1),
        )
        .expect_err("snapshot identity must match generation-zero reservation");
        assert_eq!(mismatched.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            mismatched.to_string(),
            "snapshot does not match reserved world-generation identity"
        );

        let unsupported = validate_matching_worldgen_versions(
            WORLDGEN_VERSION.saturating_add(1),
            WORLDGEN_VERSION.saturating_add(1),
        )
        .expect_err("matching persisted identities still require build compatibility");
        assert_eq!(unsupported.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            unsupported.to_string(),
            "saved world uses an incompatible world-generation version"
        );
    }
}
