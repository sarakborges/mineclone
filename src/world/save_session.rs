use std::{io, time::Instant};

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        block::BlockRegistry, day_night_cycle::DayNightCycleRegistry,
        dimension::DimensionRegistry, fluid::FluidRegistry, tool::ToolRegistry,
    },
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
    save_catalog::{SaveRegistries, SavedPlayer, SnapshotSource, WorldSnapshot, save_world},
    seed::WorldSeed,
};

const AUTOSAVE_SECONDS: f32 = 60.0;

/// The clock is deliberately excluded: passing simulation ticks alone must
/// never create another full on-disk snapshot.
#[derive(Clone, PartialEq)]
struct SavedWorldState {
    seed: u64,
    dimension_id: String,
    spawn_biome: Option<String>,
    ticks_per_second: u32,
    world_revision: u64,
    position: [f32; 3],
    creative: bool,
    inventory: Vec<Option<String>>,
}

#[derive(Resource)]
pub(crate) struct WorldSession {
    pub(crate) id: Option<String>,
    pub(crate) pending_clock: Option<(u64, u64)>,
    timer: Timer,
    first_save_done: bool,
    baseline_loaded_save: bool,
    last_saved_state: Option<SavedWorldState>,
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
            ticks_per_second: captured.ticks_per_second,
            world_revision: snapshot.world.save_content_revision(),
            position: player.position,
            creative: player.creative,
            inventory: captured.inventory.clone(),
        };
        let publication_started = Instant::now();
        save_world(
            &captured,
            SaveRegistries {
                blocks: &snapshot.blocks,
                fluids: &snapshot.fluids,
                tools: &snapshot.tools,
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
    blocks: Res<'w, BlockRegistry>,
    fluids: Res<'w, FluidRegistry>,
    tools: Res<'w, ToolRegistry>,
    dimensions: Res<'w, DimensionRegistry>,
    cycles: Res<'w, DayNightCycleRegistry>,
    player: Query<'w, 's, (&'static PlayerId, &'static Transform, &'static GameMode), With<GameplayCamera>>,
}

impl WorldSaveContext<'_, '_> {
    fn saved_state(&self) -> io::Result<SavedWorldState> {
        let (_, transform, mode) = self.player.single().map_err(|error| {
            io::Error::other(format!("cannot save world without exactly one player: {error}"))
        })?;
        let position = transform.translation;
        Ok(SavedWorldState {
            seed: self.seed.0,
            dimension_id: self.dimension.id.clone(),
            spawn_biome: self.save.spawn_biome().map(str::to_owned),
            ticks_per_second: self.rules.ticks_per_second(),
            world_revision: self.world.save_content_revision(),
            position: [position.x, position.y, position.z],
            creative: *mode == GameMode::Creative,
            inventory: self.inventory.saved_items(),
        })
    }

    fn capture(&self, id: &str) -> io::Result<WorldSnapshot> {
        let (_, transform, mode) = self.player.single().map_err(|error| {
            io::Error::other(format!("cannot save world without exactly one player: {error}"))
        })?;
        let position = transform.translation;
        WorldSnapshot::capture(SnapshotSource {
            id,
            seed: self.seed.0,
            dimension_id: &self.dimension.id,
            spawn_biome: self.save.spawn_biome(),
            ticks_per_second: self.rules.ticks_per_second(),
            player: Some(SavedPlayer {
                position: [position.x, position.y, position.z],
                creative: *mode == GameMode::Creative,
            }),
            day: self.clock.day,
            tick_in_day: self.clock.tick_in_day(),
            inventory: self.inventory.saved_items(),
            world: &self.world,
            fluids: &self.fluids,
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
    // Loading has already restored the disk state before entering Gameplay.
    // Capture its baseline on the first gameplay frame, not after a full minute:
    // otherwise genuine edits made during that minute could be skipped.
    if session.baseline_loaded_save {
        match snapshot.saved_state() {
            Ok(state) => {
                session.last_saved_state = Some(state);
                session.baseline_loaded_save = false;
            }
            Err(error) => {
                error!("Cannot initialize loaded save state: {error}");
                return;
            }
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
    if let Err(error) = session.persist(&snapshot) {
        error!("World autosave failed; the world stays loaded: {error}");
        // Retry on the next interval, not every frame. A failed first save has
        // no successful baseline, so it cannot be silently classified as clean.
        session.first_save_done = true;
        session.timer.reset();
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

pub(crate) fn autosave_only_in_gameplay(state: Res<State<GameState>>) -> bool {
    *state.get() == GameState::Gameplay
}
