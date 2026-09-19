use std::{io, time::Instant};

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};

use crate::{
    app::game_state::GameState,
    content::{
        block::BlockRegistry, creature::CreatureRegistry,
        day_night_cycle::DayNightCycleRegistry, dimension::DimensionRegistry,
        fluid::FluidRegistry, tool::ToolRegistry,
    },
    creatures::{CreatureInstance, PendingCreatureRestores, SavedCreature},
    entity::EntityHealth,
    player::{
        camera::GameplayCamera, game_mode::GameMode, hotbar::PlayerHotbar,
        player_id::PlayerId,
    },
    voxel::world::VoxelWorld,
};

use super::{
    InMemoryWorldSave,
    current_context::CurrentDimensionContext,
    day_night::DayNightClock,
    dimension::CurrentDimension,
    game_rules::GameRules,
    fluid_updates::PendingFluidUpdates,
    save_catalog::{
        PruneRegistries, SaveRegistries, SavedPlayer, SnapshotSource, WorldSnapshot, save_world,
        save_world_owned,
    },
    seed::WorldSeed,
    tick::WorldTickClock,
};

const AUTOSAVE_SECONDS: f32 = 60.0;

/// The clock is deliberately excluded: passing simulation ticks alone must
/// never create another full on-disk snapshot.
#[derive(Clone, PartialEq)]
struct SavedWorldState {
    seed: u64,
    dimension_id: String,
    spawn_biome: Option<String>,
    biome_size_multiplier: f32,
    ticks_per_second: u32,
    world_revision: u64,
    position: [f32; 3],
    creative: bool,
    health: f32,
    inventory: Vec<Option<String>>,
    selected_hotbar_slot: usize,
    creatures: Vec<SavedCreature>,
}

#[derive(Resource)]
pub(crate) struct WorldSession {
    pub(crate) id: Option<String>,
    pub(crate) pending_clock: Option<(u64, u64)>,
    timer: Timer,
    first_save_done: bool,
    baseline_loaded_save: bool,
    last_saved_state: Option<SavedWorldState>,
    autosave_task: Option<Task<AutosaveResult>>,
}

impl Default for WorldSession {
    fn default() -> Self {
        Self {
            id: None,
            pending_clock: None,
            timer: Timer::from_seconds(AUTOSAVE_SECONDS, TimerMode::Repeating),
            first_save_done: false,
            baseline_loaded_save: false,
            last_saved_state: None,
            autosave_task: None,
        }
    }
}

struct AutosaveResult {
    state: SavedWorldState,
    result: io::Result<()>,
}

struct OwnedWorldSaveCapture {
    snapshot: WorldSnapshot,
    state: SavedWorldState,
    registries: PruneRegistries,
}

impl OwnedWorldSaveCapture {
    fn persist(self) -> AutosaveResult {
        let result = save_world_owned(&self.snapshot, self.registries).map(|_| ());

        AutosaveResult {
            state: self.state,
            result,
        }
    }
}

impl WorldSession {
    pub(crate) fn new(id: String) -> Self {
        Self { id: Some(id), ..Self::default() }
    }

    pub(crate) fn loaded(id: String, day: u64, tick_in_day: u64) -> Self {
        Self {
            id: Some(id),
            pending_clock: Some((day, tick_in_day)),
            first_save_done: true,
            baseline_loaded_save: true,
            ..Self::default()
        }
    }

    pub(crate) fn persist(&mut self, snapshot: &WorldSaveContext<'_, '_>) -> io::Result<()> {
        let id = self.id.as_deref().ok_or_else(|| io::Error::other("no active world"))?;
        let capture_started = Instant::now();
        let captured = snapshot.capture(id)?;
        let capture_elapsed = capture_started.elapsed();
        // Derive the successful baseline from the EXACT snapshot being written.
        // Calling saved_state() here used to query the player and inventory a
        // second time, independently of the captured save.
        let player = captured
            .player
            .as_ref()
            .ok_or_else(|| io::Error::other("cannot save world without player state"))?;
        let state = SavedWorldState {
            seed: captured.seed,
            dimension_id: captured.dimension_id.clone(),
            spawn_biome: captured.spawn_biome.clone(),
            biome_size_multiplier: captured.biome_size_multiplier,
            ticks_per_second: captured.ticks_per_second,
            world_revision: snapshot.world.save_content_revision(),
            position: player.position,
            creative: player.creative,
            health: player.health.unwrap_or_default(),
            inventory: captured.inventory.clone(),
            selected_hotbar_slot: captured.selected_hotbar_slot,
            creatures: captured.creatures.clone(),
        };
        let publication_started = Instant::now();
        save_world(
            &captured,
            SaveRegistries {
                blocks: &snapshot.blocks,
                fluids: &snapshot.fluids,
                tools: &snapshot.tools,
                creatures: &snapshot.creature_definitions,
                dimensions: &snapshot.dimensions,
                cycles: &snapshot.cycles,
            },
        )?;
        let publication_elapsed = publication_started.elapsed();
        // Measured on the machine running the game, not inferred from CI.
        // Publication includes JSON serialization, fsync and cleanup dispatch;
        // it still blocks Leave World/Exit until the save is committed.
        info!(
            "World {id} saved: capture={capture_elapsed:?}, publication={publication_elapsed:?}"
        );
        self.last_saved_state = Some(state);
        self.baseline_loaded_save = false;
        self.first_save_done = true;
        self.timer.reset();
        Ok(())
    }
}

#[derive(SystemParam)]
pub(crate) struct WorldSaveContext<'w, 's> {
    seed: Res<'w, WorldSeed>,
    dimension: Res<'w, CurrentDimension>,
    rules: Res<'w, GameRules>,
    save: Res<'w, InMemoryWorldSave>,
    clock: Res<'w, DayNightClock>,
    inventory: Res<'w, PlayerHotbar>,
    world: Res<'w, VoxelWorld>,
    pending_fluids: Res<'w, PendingFluidUpdates>,
    world_ticks: Res<'w, WorldTickClock>,
    blocks: Res<'w, BlockRegistry>,
    fluids: Res<'w, FluidRegistry>,
    tools: Res<'w, ToolRegistry>,
    creature_definitions: Res<'w, CreatureRegistry>,
    dimensions: Res<'w, DimensionRegistry>,
    cycles: Res<'w, DayNightCycleRegistry>,
    player: Query<
        'w,
        's,
        (
            &'static PlayerId,
            &'static Transform,
            &'static GameMode,
            &'static EntityHealth,
        ),
        With<GameplayCamera>,
    >,
    creatures: Query<
        'w,
        's,
        (&'static CreatureInstance, &'static Transform, &'static EntityHealth),
    >,
    pending_creatures: Res<'w, PendingCreatureRestores>,
}

impl WorldSaveContext<'_, '_> {
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

    fn saved_state(&self) -> io::Result<SavedWorldState> {
        let (_, transform, mode, health) = self.player.single().map_err(|error| {
            io::Error::other(format!("cannot save world without exactly one player: {error}"))
        })?;
        let position = transform.translation;
        Ok(SavedWorldState {
            seed: self.seed.0,
            dimension_id: self.dimension.id.clone(),
            spawn_biome: self.save.spawn_biome().map(str::to_owned),
            biome_size_multiplier: self.save.biome_size_multiplier(),
            ticks_per_second: self.rules.ticks_per_second(),
            world_revision: self.world.save_content_revision(),
            position: [position.x, position.y, position.z],
            creative: *mode == GameMode::Creative,
            health: health.current(),
            inventory: self.inventory.saved_items(),
            selected_hotbar_slot: self.inventory.selected_slot(),
            creatures: self.saved_creatures(),
        })
    }

    fn capture_owned(&self, id: &str) -> io::Result<OwnedWorldSaveCapture> {
        let (_, transform, mode, health) = self.player.single().map_err(|error| {
            io::Error::other(format!("cannot save world without exactly one player: {error}"))
        })?;
        let position = transform.translation;
        let player = SavedPlayer {
            position: [position.x, position.y, position.z],
            creative: *mode == GameMode::Creative,
            health: Some(health.current()),
        };
        let inventory = self.inventory.saved_items();
        let selected_hotbar_slot = self.inventory.selected_slot();
        let creatures = self.saved_creatures();
        let snapshot = WorldSnapshot::capture(SnapshotSource {
            id,
            seed: self.seed.0,
            dimension_id: &self.dimension.id,
            spawn_biome: self.save.spawn_biome(),
            biome_size_multiplier: self.save.biome_size_multiplier(),
            ticks_per_second: self.rules.ticks_per_second(),
            player: Some(player.clone()),
            day: self.clock.day,
            tick_in_day: self.clock.tick_in_day(),
            inventory: inventory.clone(),
            selected_hotbar_slot,
            world: &self.world,
            fluids: &self.fluids,
            pending_fluids: &self.pending_fluids,
            world_tick: self.world_ticks.current_tick(),
            creatures: creatures.clone(),
        })?;
        let state = SavedWorldState {
            seed: self.seed.0,
            dimension_id: self.dimension.id.clone(),
            spawn_biome: self.save.spawn_biome().map(str::to_owned),
            biome_size_multiplier: self.save.biome_size_multiplier(),
            ticks_per_second: self.rules.ticks_per_second(),
            world_revision: self.world.save_content_revision(),
            position: player.position,
            creative: player.creative,
            health: player.health.unwrap_or(health.current()),
            inventory,
            selected_hotbar_slot,
            creatures,
        };

        Ok(OwnedWorldSaveCapture {
            snapshot,
            state,
            registries: SaveRegistries {
                blocks: &self.blocks,
                fluids: &self.fluids,
                tools: &self.tools,
                creatures: &self.creature_definitions,
                dimensions: &self.dimensions,
                cycles: &self.cycles,
            }
            .owned_for_pruning(),
        })
    }

    fn capture(&self, id: &str) -> io::Result<WorldSnapshot> {
        let (_, transform, mode, health) = self.player.single().map_err(|error| {
            io::Error::other(format!("cannot save world without exactly one player: {error}"))
        })?;
        let position = transform.translation;
        WorldSnapshot::capture(SnapshotSource {
            id,
            seed: self.seed.0,
            dimension_id: &self.dimension.id,
            spawn_biome: self.save.spawn_biome(),
            biome_size_multiplier: self.save.biome_size_multiplier(),
            ticks_per_second: self.rules.ticks_per_second(),
            player: Some(SavedPlayer {
                position: [position.x, position.y, position.z],
                creative: *mode == GameMode::Creative,
                health: Some(health.current()),
            }),
            day: self.clock.day,
            tick_in_day: self.clock.tick_in_day(),
            inventory: self.inventory.saved_items(),
            selected_hotbar_slot: self.inventory.selected_slot(),
            world: &self.world,
            fluids: &self.fluids,
            pending_fluids: &self.pending_fluids,
            world_tick: self.world_ticks.current_tick(),
            creatures: self.saved_creatures(),
        })
    }
}

pub(crate) fn autosave_world(
    time: Res<Time<Real>>,
    mut session: ResMut<WorldSession>,
    snapshot: WorldSaveContext,
) {
    if session.id.is_none() {
        return;
    }

    let completed = session
        .autosave_task
        .as_mut()
        .and_then(check_ready);
    if let Some(completed) = completed {
        session.autosave_task = None;
        match completed.result {
            Ok(()) => {
                session.last_saved_state = Some(completed.state);
                session.first_save_done = true;
            }
            Err(error) => {
                error!("World autosave failed; the world stays loaded: {error}");
            }
        }
        session.timer.reset();
    }
    if session.autosave_task.is_some() {
        return;
    }

    // Loading restores disk state with world revision zero. Streaming runs before
    // this Last-stage system, so a newly generated chunk on the first gameplay
    // frame can already have advanced the persistent revision. Never absorb that
    // new chunk into the disk baseline.
    if session.baseline_loaded_save {
        let state = match snapshot.saved_state() {
            Ok(state) => state,
            Err(error) => {
                error!("Cannot initialize loaded save state: {error}");
                return;
            }
        };
        session.baseline_loaded_save = false;
        if state.world_revision == 0 {
            session.last_saved_state = Some(state);
            return;
        }
    }

    if session.first_save_done && !session.timer.tick(time.delta()).just_finished() {
        return;
    }

    if let Some(previous) = &session.last_saved_state {
        match snapshot.saved_state() {
            Ok(current) if current == *previous => return,
            Ok(_) => {}
            Err(error) => {
                error!("Cannot inspect autosave state: {error}");
                return;
            }
        }
    }

    let id = session
        .id
        .as_deref()
        .expect("active world ID checked above");
    let captured = match snapshot.capture_owned(id) {
        Ok(captured) => captured,
        Err(error) => {
            error!("Cannot capture world autosave state: {error}");
            session.first_save_done = true;
            session.timer.reset();
            return;
        }
    };

    session.autosave_task = Some(AsyncComputeTaskPool::get().spawn(async move {
        captured.persist()
    }));
    session.first_save_done = true;
    session.timer.reset();
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

pub(crate) fn autosave_only_in_gameplay(state: Res<State<GameState>>) -> bool {
    *state.get() == GameState::Gameplay
}
