use std::io;

use serde::{Deserialize, Serialize};

use crate::{
    content::fluid::FluidRegistry,
    creatures::SavedCreature,
    voxel::{chunk_disk::DiskChunk, world::VoxelWorld},
};

use super::invalid_data;
use crate::world::{
    fluid_updates::{PendingFluidUpdates, SavedFluidUpdates},
    new_world::{
        DEFAULT_BIOME_SIZE_MULTIPLIER, WorldgenVersion, is_valid_biome_size_multiplier,
        legacy_worldgen_version,
    },
    world_names::validate_world_name,
};

pub(super) const SAVE_FORMAT_VERSION: u32 = 1;

fn default_saved_biome_size_multiplier() -> f32 {
    DEFAULT_BIOME_SIZE_MULTIPLIER
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct WorldManifest {
    pub(super) format_version: u32,
    pub(super) id: String,
    pub(super) seed: u64,
    pub(super) dimension_id: String,
    #[serde(default = "legacy_worldgen_version")]
    pub(super) worldgen_version: WorldgenVersion,
    #[serde(default = "default_saved_biome_size_multiplier")]
    pub(super) biome_size_multiplier: f32,
    pub(super) ticks_per_second: u32,
    pub(super) last_saved_unix_ms: u64,
    #[serde(default)]
    pub(super) generation: u64,
    #[serde(default)]
    pub(super) snapshot_file: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SavedPlayer {
    pub(crate) position: [f32; 3],
    pub(crate) creative: bool,
    #[serde(default)]
    pub(crate) health: Option<f32>,
    #[serde(default)]
    pub(crate) yaw: f32,
    #[serde(default)]
    pub(crate) pitch: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WorldSnapshot {
    pub(super) format_version: u32,
    pub(crate) id: String,
    pub(crate) seed: u64,
    pub(crate) dimension_id: String,
    #[serde(default = "legacy_worldgen_version")]
    pub(super) worldgen_version: WorldgenVersion,
    #[serde(default)]
    pub(crate) spawn_biome: Option<String>,
    #[serde(default = "default_saved_biome_size_multiplier")]
    pub(crate) biome_size_multiplier: f32,
    pub(crate) ticks_per_second: u32,
    pub(crate) player: Option<SavedPlayer>,
    pub(crate) day: u64,
    pub(crate) tick_in_day: u64,
    pub(crate) inventory: Vec<Option<String>>,
    #[serde(default)]
    pub(crate) selected_hotbar_slot: usize,
    #[serde(default)]
    pub(crate) fluid_updates: SavedFluidUpdates,
    #[serde(default)]
    pub(crate) creatures: Vec<SavedCreature>,
    pub(super) chunks: Vec<DiskChunk>,
}

pub(crate) struct SnapshotSource<'a> {
    pub(crate) id: &'a str,
    pub(crate) seed: u64,
    pub(crate) dimension_id: &'a str,
    pub(crate) spawn_biome: Option<&'a str>,
    pub(crate) biome_size_multiplier: f32,
    pub(crate) ticks_per_second: u32,
    pub(crate) player: Option<SavedPlayer>,
    pub(crate) day: u64,
    pub(crate) tick_in_day: u64,
    pub(crate) inventory: Vec<Option<String>>,
    pub(crate) selected_hotbar_slot: usize,
    pub(crate) world: &'a VoxelWorld,
    pub(crate) fluids: &'a FluidRegistry,
    pub(crate) pending_fluids: &'a PendingFluidUpdates,
    pub(crate) world_tick: u64,
    pub(crate) creatures: Vec<SavedCreature>,
}

impl WorldSnapshot {
    pub(crate) fn capture(source: SnapshotSource<'_>) -> io::Result<Self> {
        validate_world_name(source.id)?;
        if source.ticks_per_second == 0 || source.dimension_id.is_empty() || source.day == 0 {
            return Err(invalid_data("incomplete world state"));
        }
        if !is_valid_biome_size_multiplier(source.biome_size_multiplier) {
            return Err(invalid_data("invalid biome size multiplier"));
        }
        if let Some(player) = source.player.as_ref() {
            if player.position.iter().any(|coord| !coord.is_finite()) {
                return Err(invalid_data("player position must be finite"));
            }
            if !player.yaw.is_finite() || !player.pitch.is_finite() {
                return Err(invalid_data("player look must be finite"));
            }
        }

        Ok(Self {
            format_version: SAVE_FORMAT_VERSION,
            id: source.id.to_owned(),
            seed: source.seed,
            dimension_id: source.dimension_id.to_owned(),
            worldgen_version: WorldgenVersion::current(),
            spawn_biome: source.spawn_biome.map(str::to_owned),
            biome_size_multiplier: source.biome_size_multiplier,
            ticks_per_second: source.ticks_per_second,
            player: source.player,
            day: source.day,
            tick_in_day: source.tick_in_day,
            inventory: source.inventory,
            selected_hotbar_slot: source.selected_hotbar_slot,
            fluid_updates: source
                .pending_fluids
                .capture_saved(source.world_tick, source.fluids)?,
            creatures: source.creatures,
            chunks: source.world.save_persistent_chunks(source.fluids)?,
        })
    }
}
