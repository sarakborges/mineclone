use std::{collections::HashMap, io};

use bevy::prelude::*;

use crate::{
    creatures::PendingCreatureRestores,
    gameplay::storage_box::StorageBoxStorage,
    player::{
        game_mode::GameMode,
        hotbar::PlayerHotbar,
        player_id::LOCAL_PLAYER_ID,
    },
    voxel::world::VoxelWorld,
    world::{
        InMemoryWorldSave, WorldLoadMode, WorldSeed,
        dimension::CurrentDimension,
        dimension_persistence::{InactiveDimensionState, InactiveDimensionStates},
        fluid_updates::PendingFluidUpdates,
        game_rules::GameRules,
        save_catalog::{SaveRegistries, WorldDirectoryLock, WorldSnapshot},
        save_session::WorldSession,
    },
};

pub(super) enum WorldActivationError {
    Load(io::Error),
    Inventory(io::Error),
}

pub(super) struct PreparedWorldActivation {
    inventory: PlayerHotbar,
    storage_boxes: StorageBoxStorage,
    save: InMemoryWorldSave,
    session_lock: WorldDirectoryLock,
    pending_fluids: PendingFluidUpdates,
    pending_creatures: PendingCreatureRestores,
    inactive_dimensions: InactiveDimensionStates,
    seed: WorldSeed,
    dimension: CurrentDimension,
    rules: GameRules,
    world: VoxelWorld,
    session: WorldSession,
}

impl PreparedWorldActivation {
    pub(super) fn prepare(
        id: String,
        mut snapshot: WorldSnapshot,
        world: VoxelWorld,
        mut inactive_worlds: HashMap<String, VoxelWorld>,
        session_lock: WorldDirectoryLock,
        registries: SaveRegistries<'_>,
    ) -> Result<Self, WorldActivationError> {
        let pending_fluids =
            PendingFluidUpdates::from_saved(&snapshot.fluid_updates, registries.fluids)
                .map_err(WorldActivationError::Load)?;
        let inventory = PlayerHotbar::from_saved_items_and_selection(
            &snapshot.inventory,
            snapshot.selected_hotbar_slot,
            registries.items,
            registries.blocks,
            registries.layers,
            registries.objects,
            registries.tools,
        )
        .map_err(WorldActivationError::Inventory)?;
        let storage_boxes = StorageBoxStorage::from_saved_boxes(
            &snapshot.storage_boxes,
            registries.items,
            registries.blocks,
            registries.layers,
            registries.objects,
            registries.tools,
        )
        .map_err(WorldActivationError::Inventory)?;

        let mut inactive_dimensions = InactiveDimensionStates::default();
        for saved in std::mem::take(&mut snapshot.inactive_dimensions) {
            let inactive_world = inactive_worlds.remove(&saved.dimension_id).ok_or_else(|| {
                WorldActivationError::Load(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("saved dimension world is missing: {}", saved.dimension_id),
                ))
            })?;
            let storage = StorageBoxStorage::from_saved_boxes(
                &saved.storage_boxes,
                registries.items,
                registries.blocks,
                registries.layers,
                registries.objects,
                registries.tools,
            )
            .map_err(WorldActivationError::Inventory)?;
            inactive_dimensions.insert(
                saved.dimension_id,
                InactiveDimensionState::new(
                    inactive_world,
                    saved.spawn_biome,
                    storage,
                    saved.fluid_updates,
                    saved.creatures,
                ),
            );
        }
        if !inactive_worlds.is_empty() {
            return Err(WorldActivationError::Load(io::Error::new(
                io::ErrorKind::InvalidData,
                "save loaded dimension worlds that are absent from snapshot metadata",
            )));
        }

        let seed = WorldSeed(snapshot.seed);
        let dimension_id = snapshot.dimension_id.clone();
        let mut rules = GameRules::default();
        rules.set_ticks_per_second(snapshot.ticks_per_second);
        rules.set_spawn_creatures(snapshot.spawn_creatures);

        let mut save = InMemoryWorldSave::default();
        save.begin_new_world(
            seed,
            &dimension_id,
            rules,
            snapshot.spawn_biome.as_deref(),
            snapshot.biome_size_multiplier,
            snapshot.world_generation,
        );
        if let Some(player) = snapshot.player {
            let game_mode = if player.spectator {
                GameMode::Spectator
            } else if player.creative {
                GameMode::Creative
            } else {
                GameMode::Survival
            };
            save.save_player_state_with_health(
                LOCAL_PLAYER_ID,
                Vec3::from_array(player.position),
                game_mode,
                player.health,
                Some((player.yaw, player.pitch)),
                player.flying,
            );
        }

        Ok(Self {
            inventory,
            storage_boxes,
            save,
            session_lock,
            pending_fluids,
            pending_creatures: PendingCreatureRestores::new(std::mem::take(
                &mut snapshot.creatures,
            )),
            inactive_dimensions,
            seed,
            dimension: CurrentDimension {
                id: dimension_id.into(),
            },
            rules,
            world,
            session: WorldSession::loaded(id, snapshot.day, snapshot.tick_in_day),
        })
    }

    pub(super) fn commit(
        self,
        commands: &mut Commands,
        inventory: &mut PlayerHotbar,
        save: &mut InMemoryWorldSave,
    ) {
        *inventory = self.inventory;
        *save = self.save;
        commands.insert_resource(self.storage_boxes);
        commands.insert_resource(self.session_lock);
        commands.insert_resource(self.pending_fluids);
        commands.insert_resource(self.pending_creatures);
        commands.insert_resource(self.inactive_dimensions);
        commands.insert_resource(self.seed);
        commands.insert_resource(self.dimension);
        commands.insert_resource(self.rules);
        commands.insert_resource(self.world);
        commands.insert_resource(self.session);
        commands.insert_resource(WorldLoadMode::Load);
    }
}
