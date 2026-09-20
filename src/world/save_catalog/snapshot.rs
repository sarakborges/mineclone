use std::io;

use serde::{Deserialize, Serialize};

use crate::{
    content::fluid::FluidRegistry,
    creatures::SavedCreature,
    voxel::world::VoxelWorld,
};

use super::{chunks::SavedChunkCatalog, invalid_data};
use crate::world::{
    fluid_updates::{PendingFluidUpdates, SavedFluidUpdates},
    new_world::{
        DEFAULT_BIOME_SIZE_MULTIPLIER, WorldgenVersion, is_valid_biome_size_multiplier,
        legacy_worldgen_version,
    },
    world_names::validate_world_name,
};

pub(super) const LEGACY_SAVE_FORMAT_VERSION: u32 = 1;
pub(super) const SAVE_FORMAT_VERSION: u32 = 2;

pub(super) fn is_supported_save_format(version: u32) -> bool {
    matches!(version, LEGACY_SAVE_FORMAT_VERSION | SAVE_FORMAT_VERSION)
}

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

#[derive(Clone, Debug)]
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
    pub(super) chunks: SavedChunkCatalog,
}

#[derive(Debug, Deserialize)]
pub(super) struct StoredWorldSnapshot {
    pub(super) format_version: u32,
    pub(super) id: String,
    pub(super) seed: u64,
    pub(super) dimension_id: String,
    #[serde(default = "legacy_worldgen_version")]
    pub(super) worldgen_version: WorldgenVersion,
    #[serde(default)]
    pub(super) spawn_biome: Option<String>,
    #[serde(default = "default_saved_biome_size_multiplier")]
    pub(super) biome_size_multiplier: f32,
    pub(super) ticks_per_second: u32,
    pub(super) player: Option<SavedPlayer>,
    pub(super) day: u64,
    pub(super) tick_in_day: u64,
    pub(super) inventory: Vec<Option<String>>,
    #[serde(default)]
    pub(super) selected_hotbar_slot: usize,
    #[serde(default)]
    pub(super) fluid_updates: SavedFluidUpdates,
    #[serde(default)]
    pub(super) creatures: Vec<SavedCreature>,
    #[serde(default)]
    chunks: Option<SavedChunkCatalog>,
}

#[derive(Serialize)]
pub(super) struct WorldSnapshotV2<'a> {
    format_version: u32,
    id: &'a str,
    seed: u64,
    dimension_id: &'a str,
    worldgen_version: &'a WorldgenVersion,
    spawn_biome: &'a Option<String>,
    biome_size_multiplier: f32,
    ticks_per_second: u32,
    player: &'a Option<SavedPlayer>,
    day: u64,
    tick_in_day: u64,
    inventory: &'a [Option<String>],
    selected_hotbar_slot: usize,
    fluid_updates: &'a SavedFluidUpdates,
    creatures: &'a [SavedCreature],
}

impl StoredWorldSnapshot {
    pub(super) fn into_runtime(self) -> io::Result<(WorldSnapshot, Option<SavedChunkCatalog>)> {
        let inline_chunks = match self.format_version {
            LEGACY_SAVE_FORMAT_VERSION => Some(
                self.chunks
                    .ok_or_else(|| invalid_data("format v1 snapshot is missing inline chunks"))?,
            ),
            SAVE_FORMAT_VERSION => {
                if self.chunks.is_some() {
                    return Err(invalid_data(
                        "format v2 snapshot must not contain inline chunks",
                    ));
                }
                None
            }
            _ => return Err(invalid_data("unsupported snapshot format")),
        };

        Ok((
            WorldSnapshot {
                format_version: SAVE_FORMAT_VERSION,
                id: self.id,
                seed: self.seed,
                dimension_id: self.dimension_id,
                worldgen_version: self.worldgen_version,
                spawn_biome: self.spawn_biome,
                biome_size_multiplier: self.biome_size_multiplier,
                ticks_per_second: self.ticks_per_second,
                player: self.player,
                day: self.day,
                tick_in_day: self.tick_in_day,
                inventory: self.inventory,
                selected_hotbar_slot: self.selected_hotbar_slot,
                fluid_updates: self.fluid_updates,
                creatures: self.creatures,
                chunks: SavedChunkCatalog::default(),
            },
            inline_chunks,
        ))
    }
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
    pub(super) fn disk_v2(&self) -> WorldSnapshotV2<'_> {
        WorldSnapshotV2 {
            format_version: SAVE_FORMAT_VERSION,
            id: &self.id,
            seed: self.seed,
            dimension_id: &self.dimension_id,
            worldgen_version: &self.worldgen_version,
            spawn_biome: &self.spawn_biome,
            biome_size_multiplier: self.biome_size_multiplier,
            ticks_per_second: self.ticks_per_second,
            player: &self.player,
            day: self.day,
            tick_in_day: self.tick_in_day,
            inventory: &self.inventory,
            selected_hotbar_slot: self.selected_hotbar_slot,
            fluid_updates: &self.fluid_updates,
            creatures: &self.creatures,
        }
    }

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
            chunks: SavedChunkCatalog::capture(source.world, source.fluids)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v2_snapshot_payload_omits_inline_chunks() {
        let snapshot = WorldSnapshot {
            format_version: SAVE_FORMAT_VERSION,
            id: "World".to_owned(),
            seed: 1,
            dimension_id: "asteria:overworld".to_owned(),
            worldgen_version: WorldgenVersion::current(),
            spawn_biome: None,
            biome_size_multiplier: DEFAULT_BIOME_SIZE_MULTIPLIER,
            ticks_per_second: 20,
            player: None,
            day: 1,
            tick_in_day: 0,
            inventory: Vec::new(),
            selected_hotbar_slot: 0,
            fluid_updates: SavedFluidUpdates::default(),
            creatures: Vec::new(),
            chunks: SavedChunkCatalog::default(),
        };

        let value = serde_json::to_value(snapshot.disk_v2()).unwrap();
        assert_eq!(value["format_version"], SAVE_FORMAT_VERSION);
        assert!(value.get("chunks").is_none());
    }
}
