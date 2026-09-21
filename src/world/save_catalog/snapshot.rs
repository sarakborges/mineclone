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
    new_world::{WorldgenVersion, is_valid_biome_size_multiplier},
    world_names::validate_world_name,
};

pub(super) const SAVE_FORMAT_VERSION: u32 = 2;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorldManifest {
    pub(super) format_version: u32,
    pub(super) id: String,
    pub(super) seed: u64,
    pub(super) dimension_id: String,
    pub(super) worldgen_version: WorldgenVersion,
    pub(super) biome_size_multiplier: f32,
    pub(super) ticks_per_second: u32,
    pub(super) last_saved_unix_ms: u64,
    pub(super) generation: u64,
    pub(super) snapshot_file: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedPlayer {
    pub(crate) position: [f32; 3],
    pub(crate) creative: bool,
    pub(crate) health: Option<f32>,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct WorldSnapshot {
    pub(crate) id: String,
    pub(crate) seed: u64,
    pub(crate) dimension_id: String,
    pub(super) worldgen_version: WorldgenVersion,
    pub(crate) spawn_biome: Option<String>,
    pub(crate) current_biome: Option<String>,
    pub(crate) biome_size_multiplier: f32,
    pub(crate) ticks_per_second: u32,
    pub(crate) player: Option<SavedPlayer>,
    pub(crate) day: u64,
    pub(crate) tick_in_day: u64,
    pub(crate) inventory: Vec<Option<String>>,
    pub(crate) selected_hotbar_slot: usize,
    pub(crate) fluid_updates: SavedFluidUpdates,
    pub(crate) creatures: Vec<SavedCreature>,
    pub(super) chunks: SavedChunkCatalog,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredWorldSnapshot {
    pub(super) format_version: u32,
    pub(super) id: String,
    pub(super) seed: u64,
    pub(super) dimension_id: String,
    pub(super) worldgen_version: WorldgenVersion,
    pub(super) spawn_biome: Option<String>,
    #[serde(default)]
    pub(super) current_biome: Option<String>,
    pub(super) biome_size_multiplier: f32,
    pub(super) ticks_per_second: u32,
    pub(super) player: Option<SavedPlayer>,
    pub(super) day: u64,
    pub(super) tick_in_day: u64,
    pub(super) inventory: Vec<Option<String>>,
    pub(super) selected_hotbar_slot: usize,
    pub(super) fluid_updates: SavedFluidUpdates,
    pub(super) creatures: Vec<SavedCreature>,
}

#[derive(Serialize)]
pub(super) struct DiskWorldSnapshot<'a> {
    format_version: u32,
    id: &'a str,
    seed: u64,
    dimension_id: &'a str,
    worldgen_version: &'a WorldgenVersion,
    spawn_biome: &'a Option<String>,
    current_biome: &'a Option<String>,
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
    pub(super) fn into_runtime(self) -> WorldSnapshot {
        WorldSnapshot {
            id: self.id,
            seed: self.seed,
            dimension_id: self.dimension_id,
            worldgen_version: self.worldgen_version,
            spawn_biome: self.spawn_biome,
            current_biome: self.current_biome,
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
        }
    }
}

pub(crate) struct SnapshotSource<'a> {
    pub(crate) id: &'a str,
    pub(crate) seed: u64,
    pub(crate) dimension_id: &'a str,
    pub(crate) spawn_biome: Option<&'a str>,
    pub(crate) current_biome: Option<&'a str>,
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
    pub(super) fn disk_snapshot(&self) -> DiskWorldSnapshot<'_> {
        DiskWorldSnapshot {
            format_version: SAVE_FORMAT_VERSION,
            id: &self.id,
            seed: self.seed,
            dimension_id: &self.dimension_id,
            worldgen_version: &self.worldgen_version,
            spawn_biome: &self.spawn_biome,
            current_biome: &self.current_biome,
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
            id: source.id.to_owned(),
            seed: source.seed,
            dimension_id: source.dimension_id.to_owned(),
            worldgen_version: WorldgenVersion::current(),
            spawn_biome: source.spawn_biome.map(str::to_owned),
            current_biome: source.current_biome.map(str::to_owned),
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
    fn current_snapshot_payload_omits_inline_chunks() {
        let snapshot = WorldSnapshot {
            id: "World".to_owned(),
            seed: 1,
            dimension_id: "asteria:overworld".to_owned(),
            worldgen_version: WorldgenVersion::current(),
            spawn_biome: None,
            current_biome: None,
            biome_size_multiplier: crate::world::new_world::DEFAULT_BIOME_SIZE_MULTIPLIER,
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

        let value = serde_json::to_value(snapshot.disk_snapshot()).unwrap();
        assert_eq!(value["format_version"], SAVE_FORMAT_VERSION);
        assert!(value.get("chunks").is_none());
    }
}
