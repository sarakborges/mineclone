use std::{io, time::Instant};

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    window::WindowCloseRequested,
};

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry, block::BlockRegistry, creature::CreatureRegistry,
        day_night_cycle::DayNightCycleRegistry, dimension::DimensionRegistry,
        fluid::FluidRegistry, layer::LayerRegistry, tool::ToolRegistry,
    },
    creatures::{CreatureInstance, PendingCreatureRestores, SavedCreature},
    entity::EntityHealth,
    player::{
        camera::GameplayCamera, game_mode::GameMode, hotbar::PlayerHotbar,
        movement::flight::FlightState, player_id::PlayerId,
    },
    voxel::world::VoxelWorld,
};

use super::{
    InMemoryWorldSave,
    biome::CurrentBiome,
    current_context::CurrentDimensionContext,
    day_night::DayNightClock,
    dimension::CurrentDimension,
    game_rules::GameRules,
    fluid_updates::PendingFluidUpdates,
    save_catalog::{SaveRegistries, SavedPlayer, SnapshotSource, WorldSnapshot, save_world},
    seed::WorldSeed,
    tick::WorldTickClock,
};

#[derive(Resource, Default)]
pub(crate) struct WorldSession {
    pub(crate) id: Option<String>,
    pub(crate) pending_clock: Option<(u64, u64)>,
}

impl WorldSession {
    pub(crate) fn new(id: String) -> Self {
        Self {
            id: Some(id),
            ..Self::default()
        }
    }

    pub(crate) fn loaded(id: String, day: u64, tick_in_day: u64) -> Self {
        Self {
            id: Some(id),
            pending_clock: Some((day, tick_in_day)),
        }
    }

    pub(crate) fn persist(&self, snapshot: &WorldSaveContext<'_, '_>) -> io::Result<()> {
        let id = self
            .id
            .as_deref()
            .ok_or_else(|| io::Error::other("no active world"))?;
        let capture_started = Instant::now();
        let captured = snapshot.capture(id)?;
        let capture_elapsed = capture_started.elapsed();
        let publication_started = Instant::now();
        save_world(&captured, snapshot.registries.for_validation())?;
        let publication_elapsed = publication_started.elapsed();
        // Measured on the machine running the game, not inferred from CI.
        // Publication includes JSON serialization, fsync and cleanup dispatch;
        // final world exit remains blocked until the commit is durable.
        info!(
            "World {id} saved: capture={capture_elapsed:?}, publication={publication_elapsed:?}"
        );
        Ok(())
    }
}

#[derive(SystemParam)]
struct WorldSnapshotState<'w> {
    seed: Res<'w, WorldSeed>,
    dimension: Res<'w, CurrentDimension>,
    rules: Res<'w, GameRules>,
    save: Res<'w, InMemoryWorldSave>,
    biome: Res<'w, CurrentBiome>,
    clock: Res<'w, DayNightClock>,
    inventory: Res<'w, PlayerHotbar>,
    world: Res<'w, VoxelWorld>,
    pending_fluids: Res<'w, PendingFluidUpdates>,
    world_ticks: Res<'w, WorldTickClock>,
}

type SavedPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static GameMode,
        &'static EntityHealth,
        &'static GameplayCamera,
        &'static FlightState,
    ),
    (With<GameplayCamera>, With<PlayerId>),
>;

type SavedCreatureQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static CreatureInstance,
        &'static Transform,
        &'static EntityHealth,
    ),
>;

#[derive(SystemParam)]
struct WorldSaveEntities<'w, 's> {
    player: SavedPlayerQuery<'w, 's>,
    creatures: SavedCreatureQuery<'w, 's>,
    pending_creatures: Res<'w, PendingCreatureRestores>,
}

impl WorldSaveEntities<'_, '_> {
    fn saved_player(&self) -> io::Result<SavedPlayer> {
        let (transform, mode, health, camera, flight) = self.player.single().map_err(|error| {
            io::Error::other(format!(
                "cannot save world without exactly one player: {error}"
            ))
        })?;
        let position = transform.translation;
        Ok(SavedPlayer {
            position: [position.x, position.y, position.z],
            creative: *mode == GameMode::Creative,
            flying: flight.is_active(),
            health: Some(health.current()),
            yaw: camera.yaw,
            pitch: camera.pitch,
        })
    }

    fn saved_creatures(&self) -> Vec<SavedCreature> {
        let mut creatures = self.pending_creatures.saved().to_vec();
        creatures.extend(
            self.creatures
                .iter()
                .filter(|(_, _, health)| !health.is_dead())
                .map(|(instance, transform, health)| SavedCreature {
                    definition_id: instance.definition_id.clone(),
                    position: transform.translation.to_array(),
                    health: health.current(),
                }),
        );
        creatures.sort_unstable_by(|left, right| {
            left.definition_id
                .cmp(&right.definition_id)
                .then_with(|| left.position[0].total_cmp(&right.position[0]))
                .then_with(|| left.position[1].total_cmp(&right.position[1]))
                .then_with(|| left.position[2].total_cmp(&right.position[2]))
        });
        creatures
    }
}

#[derive(SystemParam)]
struct WorldSaveRegistries<'w> {
    biomes: Res<'w, BiomeRegistry>,
    blocks: Res<'w, BlockRegistry>,
    layers: Res<'w, LayerRegistry>,
    fluids: Res<'w, FluidRegistry>,
    tools: Res<'w, ToolRegistry>,
    creature_definitions: Res<'w, CreatureRegistry>,
    dimensions: Res<'w, DimensionRegistry>,
    cycles: Res<'w, DayNightCycleRegistry>,
}

impl WorldSaveRegistries<'_> {
    fn for_validation(&self) -> SaveRegistries<'_> {
        SaveRegistries {
            biomes: &self.biomes,
            blocks: &self.blocks,
            layers: &self.layers,
            fluids: &self.fluids,
            tools: &self.tools,
            creatures: &self.creature_definitions,
            dimensions: &self.dimensions,
            cycles: &self.cycles,
        }
    }
}

#[derive(SystemParam)]
pub(crate) struct WorldSaveContext<'w, 's> {
    state: WorldSnapshotState<'w>,
    entities: WorldSaveEntities<'w, 's>,
    registries: WorldSaveRegistries<'w>,
}

impl WorldSaveContext<'_, '_> {
    fn capture(&self, id: &str) -> io::Result<WorldSnapshot> {
        WorldSnapshot::capture(SnapshotSource {
            id,
            seed: self.state.seed.0,
            dimension_id: &self.state.dimension.id,
            spawn_biome: self.state.save.spawn_biome(),
            current_biome: Some(self.state.biome.id.as_str()),
            biome_size_multiplier: self.state.save.biome_size_multiplier(),
            ticks_per_second: self.state.rules.ticks_per_second(),
            player: Some(self.entities.saved_player()?),
            day: self.state.clock.day,
            tick_in_day: self.state.clock.tick_in_day(),
            inventory: self.state.inventory.saved_items(),
            selected_hotbar_slot: self.state.inventory.selected_slot(),
            world: &self.state.world,
            fluids: &self.registries.fluids,
            pending_fluids: &self.state.pending_fluids,
            world_tick: self.state.world_ticks.current_tick(),
            creatures: self.entities.saved_creatures(),
        })
    }
}

pub(crate) fn restore_loaded_clock(
    mut session: ResMut<WorldSession>,
    dimension: CurrentDimensionContext,
    cycles: Res<DayNightCycleRegistry>,
    mut clock: ResMut<DayNightClock>,
) {
    let Some((day, tick_in_day)) = session.pending_clock.take() else {
        return;
    };
    let definition = dimension
        .definition()
        .expect("loaded dimension must exist in content registry");
    let cycle = cycles
        .get(&definition.day_night_cycle)
        .expect("loaded day-night cycle must exist in content registry");
    assert!(
        clock.restore(day, tick_in_day, cycle.day_duration_ticks),
        "loaded clock must be validated before entering gameplay"
    );
}

/// The default Bevy close handler is disabled so gameplay can durably save
/// before the process exits. On failure the close request is consumed and the
/// window remains open, matching Leave World / Exit Game retry semantics.
pub(crate) fn save_on_gameplay_window_close(
    mut close_requests: MessageReader<WindowCloseRequested>,
    session: Res<WorldSession>,
    snapshot: WorldSaveContext,
    mut app_exit: MessageWriter<AppExit>,
) {
    if close_requests.read().next().is_none() {
        return;
    }

    if let Err(error) = session.persist(&snapshot) {
        error!("World save failed; keeping current world open: {error}");
        return;
    }

    app_exit.write(AppExit::Success);
}

/// Keep consuming close requests while Gameplay owns the save-specific reader,
/// but exit immediately in menu/loading states where there is no active world
/// snapshot to publish.
pub(crate) fn exit_on_window_close_without_gameplay(
    state: Res<State<GameState>>,
    mut close_requests: MessageReader<WindowCloseRequested>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if close_requests.read().next().is_none() || *state.get() == GameState::Gameplay {
        return;
    }

    app_exit.write(AppExit::Success);
}
