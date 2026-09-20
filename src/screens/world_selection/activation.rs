use std::io;

use bevy::prelude::*;

use crate::{
    content::{block::BlockRegistry, fluid::FluidRegistry, tool::ToolRegistry},
    creatures::PendingCreatureRestores,
    player::{
        game_mode::GameMode,
        hotbar::PlayerHotbar,
        player_id::LOCAL_PLAYER_ID,
    },
    voxel::world::VoxelWorld,
    world::{
        InMemoryWorldSave, WorldLoadMode, WorldSeed,
        dimension::CurrentDimension,
        fluid_updates::PendingFluidUpdates,
        game_rules::GameRules,
        save_catalog::{WorldDirectoryLock, WorldSnapshot},
        save_session::WorldSession,
    },
};

pub(super) enum WorldActivationError {
    Load(io::Error),
    Inventory(io::Error),
}

pub(super) struct PreparedWorldActivation {
    inventory: PlayerHotbar,
    save: InMemoryWorldSave,
    session_lock: WorldDirectoryLock,
    pending_fluids: PendingFluidUpdates,
    pending_creatures: PendingCreatureRestores,
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
        session_lock: WorldDirectoryLock,
        blocks: &BlockRegistry,
        fluids: &FluidRegistry,
        tools: &ToolRegistry,
    ) -> Result<Self, WorldActivationError> {
        let pending_fluids =
            PendingFluidUpdates::from_saved(&snapshot.fluid_updates, fluids)
                .map_err(WorldActivationError::Load)?;
        let inventory = PlayerHotbar::from_saved_items_and_selection(
            &snapshot.inventory,
            snapshot.selected_hotbar_slot,
            blocks,
            tools,
        )
        .map_err(WorldActivationError::Inventory)?;

        let seed = WorldSeed(snapshot.seed);
        let dimension_id = snapshot.dimension_id.clone();
        let mut rules = GameRules::default();
        rules.set_ticks_per_second(snapshot.ticks_per_second);

        let mut save = InMemoryWorldSave::default();
        save.begin_new_world(
            seed,
            &dimension_id,
            rules,
            snapshot.spawn_biome.as_deref(),
            snapshot.biome_size_multiplier,
        );
        if let Some(player) = snapshot.player {
            save.save_player_state_with_health(
                LOCAL_PLAYER_ID,
                Vec3::from_array(player.position),
                if player.creative {
                    GameMode::Creative
                } else {
                    GameMode::Survival
                },
                player.health,
                Some((player.yaw, player.pitch)),
            );
        }

        Ok(Self {
            inventory,
            save,
            session_lock,
            pending_fluids,
            pending_creatures: PendingCreatureRestores::new(std::mem::take(
                &mut snapshot.creatures,
            )),
            seed,
            dimension: CurrentDimension { id: dimension_id },
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
        commands.insert_resource(self.session_lock);
        commands.insert_resource(self.pending_fluids);
        commands.insert_resource(self.pending_creatures);
        commands.insert_resource(self.seed);
        commands.insert_resource(self.dimension);
        commands.insert_resource(self.rules);
        commands.insert_resource(self.world);
        commands.insert_resource(self.session);
        commands.insert_resource(WorldLoadMode::Load);
    }
}
