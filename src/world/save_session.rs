use std::{io, time::Instant};

use bevy::{ecs::system::SystemParam, prelude::*, window::WindowCloseRequested};

use crate::{
    app::{
        crash_log::{log_gameplay_error, log_gameplay_event, log_system_error, log_system_event},
        game_state::GameState,
    },
    content::{
        biome::BiomeRegistry, block::BlockRegistry, creature::CreatureRegistry,
        day_night_cycle::DayNightCycleRegistry, dimension::DimensionRegistry, fluid::FluidRegistry,
        item::ItemRegistry, layer::LayerRegistry, object::ObjectRegistry, tool::ToolRegistry,
    },
    creatures::{
        CreatureInstance, EntityMetaTags, PendingCreatureRestores, SavedCreature,
        sort_saved_creatures,
    },
    entity::EntityHealth,
    gameplay::storage_box::StorageBoxStorage,
    player::{
        camera::GameplayCamera, game_mode::GameMode, hotbar::PlayerHotbar,
        movement::flight::FlightState, player_id::PlayerId,
    },
    voxel::world::VoxelWorld,
};

use super::{
    current_context::CurrentDimensionContext,
    day_night::DayNightClock,
    dimension::CurrentDimension,
    dimension_persistence::InactiveDimensionStates,
    fluid_updates::PendingFluidUpdates,
    game_rules::GameRules,
    save_catalog::{
        SaveRegistries, SavedDimensionState, SavedPlayer, SnapshotSource, WorldSnapshot, save_world,
    },
    seed::WorldSeed,
    thumbnail::{
        WorldThumbnailCameraQuery, WorldThumbnailCapture, WorldThumbnailCompletion,
        begin_world_thumbnail_capture,
    },
    tick::WorldTickClock,
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct WorldId(String);

impl WorldId {
    pub(crate) fn new(value: String) -> Self {
        Self(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Resource, Default)]
pub(crate) struct WorldSession {
    pub(crate) id: Option<WorldId>,
    pub(crate) pending_clock: Option<(u64, u64)>,
}

impl WorldSession {
    pub(crate) fn id(&self) -> Option<&str> {
        self.id.as_ref().map(WorldId::as_str)
    }

    pub(crate) fn new(id: String) -> Self {
        Self {
            id: Some(WorldId::new(id)),
            ..Self::default()
        }
    }

    pub(crate) fn loaded(id: String, day: u64, tick_in_day: u64) -> Self {
        Self {
            id: Some(WorldId::new(id)),
            pending_clock: Some((day, tick_in_day)),
        }
    }

    pub(crate) fn persist(&self, snapshot: &WorldSaveContext<'_, '_>) -> io::Result<()> {
        let id = self
            .id()
            .ok_or_else(|| io::Error::other("no active world"))?;
        log_gameplay_event(format!("world.save request id={id}"));

        let capture_started = Instant::now();
        let captured = match snapshot.capture(id) {
            Ok(captured) => captured,
            Err(error) => {
                log_gameplay_error(format!(
                    "world.save failed id={id} phase=capture duration_ms={:.2} error={error}",
                    capture_started.elapsed().as_secs_f64() * 1_000.0,
                ));
                return Err(error);
            }
        };
        let capture_elapsed = capture_started.elapsed();

        let publication_started = Instant::now();
        if let Err(error) = save_world(
            &captured,
            &snapshot.state.world,
            &snapshot.state.inactive_dimensions,
            &snapshot.registries.fluids,
            snapshot.registries.for_validation(),
        ) {
            log_gameplay_error(format!(
                "world.save failed id={id} phase=publication capture_ms={:.2} publication_ms={:.2} error={error}",
                capture_elapsed.as_secs_f64() * 1_000.0,
                publication_started.elapsed().as_secs_f64() * 1_000.0,
            ));
            return Err(error);
        }
        let publication_elapsed = publication_started.elapsed();

        log_gameplay_event(format!(
            "world.save success id={} capture_ms={:.2} publication_ms={:.2}",
            id,
            capture_elapsed.as_secs_f64() * 1_000.0,
            publication_elapsed.as_secs_f64() * 1_000.0,
        ));
        Ok(())
    }
}

#[derive(SystemParam)]
struct WorldSnapshotState<'w> {
    seed: Res<'w, WorldSeed>,
    dimension: Res<'w, CurrentDimension>,
    rules: Res<'w, GameRules>,
    clock: Res<'w, DayNightClock>,
    inventory: Res<'w, PlayerHotbar>,
    storage_boxes: Res<'w, StorageBoxStorage>,
    world: Res<'w, VoxelWorld>,
    inactive_dimensions: Res<'w, InactiveDimensionStates>,
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
        &'static EntityMetaTags,
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
            spectator: *mode == GameMode::Spectator,
            flying: flight.is_active(),
            health: Some(health.current()),
            yaw: camera.yaw,
            pitch: camera.pitch,
        })
    }

    fn saved_creatures(&self) -> Vec<SavedCreature> {
        self.pending_creatures.snapshot(
            self.creatures
                .iter()
                .filter_map(|(instance, transform, health, meta_tags)| {
                    SavedCreature::from_runtime(instance, transform, health, meta_tags)
                }),
        )
    }
}

#[derive(SystemParam)]
struct WorldSaveRegistries<'w> {
    biomes: Res<'w, BiomeRegistry>,
    blocks: Res<'w, BlockRegistry>,
    items: Res<'w, ItemRegistry>,
    layers: Res<'w, LayerRegistry>,
    objects: Res<'w, ObjectRegistry>,
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
            items: &self.items,
            layers: &self.layers,
            objects: &self.objects,
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
        let mut inactive_dimensions = self
            .state
            .inactive_dimensions
            .iter()
            .map(|(dimension_id, state)| {
                let mut creatures = state.creatures().to_vec();
                sort_saved_creatures(&mut creatures);
                SavedDimensionState {
                    dimension_id: dimension_id.to_owned(),
                    storage_boxes: state.storage_boxes().saved_boxes(),
                    fluid_updates: state.fluid_updates().clone(),
                    creatures,
                }
            })
            .collect::<Vec<_>>();
        inactive_dimensions
            .sort_unstable_by(|left, right| left.dimension_id.cmp(&right.dimension_id));

        WorldSnapshot::capture(SnapshotSource {
            id,
            seed: self.state.seed.0,
            dimension_id: self.state.dimension.id.as_str(),
            ticks_per_second: self.state.rules.ticks_per_second(),
            spawn_creatures: self.state.rules.spawn_creatures(),
            player: Some(self.entities.saved_player()?),
            day: self.state.clock.day,
            tick_in_day: self.state.clock.tick_in_day(),
            inventory: self.state.inventory.saved_items(),
            selected_hotbar_slot: self.state.inventory.selected_slot(),
            storage_boxes: self.state.storage_boxes.saved_boxes(),
            fluids: &self.registries.fluids,
            pending_fluids: &self.state.pending_fluids,
            world_tick: self.state.world_ticks.current_tick(),
            creatures: self.entities.saved_creatures(),
            inactive_dimensions,
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
    log_gameplay_event(format!(
        "world.clock restored day={} tick_in_day={} cycle={} day_duration_ticks={}",
        day, tick_in_day, definition.day_night_cycle, cycle.day_duration_ticks
    ));
}

pub(crate) fn save_on_gameplay_window_close(
    mut commands: Commands,
    mut close_requests: MessageReader<WindowCloseRequested>,
    session: Res<WorldSession>,
    snapshot: WorldSaveContext,
    mut thumbnail_cameras: WorldThumbnailCameraQuery,
    thumbnail_captures: Query<(), With<WorldThumbnailCapture>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if close_requests.read().next().is_none() {
        return;
    }
    log_system_event("app.close requested state=gameplay");
    if !thumbnail_captures.is_empty() {
        log_system_event("app.close deferred reason=thumbnail_capture_active");
        return;
    }

    if let Err(error) = session.persist(&snapshot) {
        error!("World save failed; keeping current world open: {error}");
        return;
    }

    let Some(world_id) = session.id() else {
        log_system_error("app.close invariant_failed reason=world_saved_without_session_id");
        app_exit.write(AppExit::Success);
        return;
    };
    log_system_event(format!(
        "app.close save_complete world={} next=thumbnail_capture",
        world_id
    ));
    begin_world_thumbnail_capture(
        &mut commands,
        &mut thumbnail_cameras,
        world_id,
        WorldThumbnailCompletion::ExitGame,
    );
}

pub(crate) fn exit_on_window_close_without_gameplay(
    state: Res<State<GameState>>,
    mut close_requests: MessageReader<WindowCloseRequested>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if close_requests.read().next().is_none() || *state.get() == GameState::Gameplay {
        return;
    }

    log_system_event(format!(
        "app.close requested state={:?} save_required=false",
        state.get()
    ));
    app_exit.write(AppExit::Success);
}
