use std::{
    io,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use bevy::{ecs::system::SystemParam, log::warn, prelude::*, ui_widgets::ScrollArea};

use crate::{
    app::game_state::GameState,
    content::{
        block::BlockRegistry, creature::CreatureRegistry,
        day_night_cycle::DayNightCycleRegistry, dimension::DimensionRegistry,
        fluid::FluidRegistry, tool::ToolRegistry,
    },
    creatures::PendingCreatureRestores,
    localization::{ActiveLanguage, UiLocalization},
    player::{game_mode::GameMode, hotbar::PlayerHotbar, player_id::LOCAL_PLAYER_ID},
    ui::{
        button::{button, ButtonVariant, COMPACT_CONTROL_HEIGHT},
        cosmic_background::{self, STAR_FIELD},
        screen, scrollbar, surface, theme,
        transition::{ScreenTransition, ScreenTransitionTarget}, typography,
    },
    voxel::world::VoxelWorld,
    world::{
        InMemoryWorldSave, WorldLoadMode, WorldSeed,
        dimension::CurrentDimension, game_rules::GameRules,
        fluid_updates::PendingFluidUpdates,
        save_catalog::{
            SaveRegistries, WorldDirectoryLock, WorldSnapshot, WorldSummary, delete_world,
            list_verified_worlds, load_world,
        },
        save_session::WorldSession,
    },
};

pub(crate) struct WorldSelectionPlugin;

impl Plugin for WorldSelectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldSelectionState>()
            .add_systems(
                OnEnter(GameState::WorldSelection),
                (refresh_world_list, spawn_world_selection).chain(),
            )
            .add_systems(OnExit(GameState::WorldSelection), abandon_world_load)
            .add_systems(
                Update,
                // Consume Back before a worker result. A completed load in the
                // same frame as Back must never activate the discarded world.
                (poll_world_scan, handle_world_selection, poll_world_load, sync_world_selection_feedback)
                    .chain()
                    .run_if(in_state(GameState::WorldSelection)),
            );
    }
}

// A worker writes once; Bevy polls the tiny result guard without waiting for IO.
type WorldScanResult = Arc<Mutex<Option<io::Result<Vec<WorldSummary>>>>>;
type WorldLoadResult = Arc<Mutex<WorldLoadSlot>>;

#[derive(Default)]
struct WorldLoadSlot {
    abandoned: bool,
    complete: bool,
    result: Option<io::Result<(WorldSnapshot, VoxelWorld, WorldDirectoryLock)>>,
}

struct PendingWorldLoad {
    id: String,
    result: WorldLoadResult,
}

impl PendingWorldLoad {
    fn abandon(&self) {
        // Cancellation and worker publication are serialized by THIS small
        // mutex. An already-completed large world is moved to a disposer rather
        // than being dropped on the input frame; a late result is discarded by
        // the original worker itself. Neither case leaves a world in the menu.
        let stale = {
            let mut slot = self.result.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            slot.abandoned = true;
            slot.result.take()
        };
        if let Some(stale) = stale
            && let Err(error) = thread::Builder::new()
                .name("asteria-discard-world".to_owned())
                .spawn(move || drop(stale))
        {
            warn!("Could not start abandoned-world cleanup worker: {error}");
        }
    }
}

#[derive(Resource, Default)]
struct WorldSelectionState {
    worlds: Vec<WorldSummary>,
    error: String,
    scan: Option<WorldScanResult>,
    loading: Option<PendingWorldLoad>,
}

#[derive(Component, Clone)]
enum WorldSelectionAction {
    Load(String),
    Delete(String),
    Back,
}

#[derive(Component)]
struct WorldListEntry(String);

#[derive(Component)]
struct SelectionError;

#[derive(Component)]
struct WorldListStatus;

#[derive(Component)]
struct WorldListContainer;

/// Content needed to validate the world catalog, not the mutable state needed
/// to activate a selected world. Keep the scan's resource access read-only.
#[derive(SystemParam)]
struct WorldSelectionScanContent<'w> {
    blocks: Res<'w, BlockRegistry>,
    fluids: Res<'w, FluidRegistry>,
    tools: Res<'w, ToolRegistry>,
    creatures: Res<'w, CreatureRegistry>,
    dimensions: Res<'w, DimensionRegistry>,
    cycles: Res<'w, DayNightCycleRegistry>,
}

/// The selected-world loader owns its content definitions; no Bevy resource
/// borrows escape into a detached thread or survive a menu/world transition.
struct OwnedLoadContent {
    blocks: BlockRegistry,
    fluids: FluidRegistry,
    tools: ToolRegistry,
    creatures: CreatureRegistry,
    dimensions: DimensionRegistry,
    cycles: DayNightCycleRegistry,
}

impl WorldSelectionScanContent<'_> {
    fn owned_for_loading(&self) -> OwnedLoadContent {
        let mut tools = ToolRegistry::default();
        for definition in self.tools.iter() {
            tools.insert(definition.clone());
        }
        let mut creatures = CreatureRegistry::default();
        for definition in self.creatures.iter() {
            creatures.insert(definition.clone());
        }
        let mut dimensions = DimensionRegistry::default();
        let mut cycles = DayNightCycleRegistry::default();
        for definition in self.dimensions.iter() {
            if let Some(cycle) = self.cycles.get(&definition.day_night_cycle) {
                cycles.insert(cycle.clone());
            }
            dimensions.insert(definition.clone());
        }
        OwnedLoadContent {
            blocks: self.blocks.clone(),
            fluids: self.fluids.clone(),
            tools,
            creatures,
            dimensions,
            cycles,
        }
    }
}

impl OwnedLoadContent {
    fn registries(&self) -> SaveRegistries<'_> {
        SaveRegistries {
            blocks: &self.blocks,
            fluids: &self.fluids,
            tools: &self.tools,
            creatures: &self.creatures,
            dimensions: &self.dimensions,
            cycles: &self.cycles,
        }
    }
}

fn refresh_world_list(
    mut state: ResMut<WorldSelectionState>,
    content: WorldSelectionScanContent,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    state.worlds.clear();
    state.error.clear();
    // Reuse an unfinished scan after a return to the menu. An abandoned load
    // likewise stays tracked until its completion is consumed, but its heavy
    // result is disposed on a worker even if the menu is never reopened.
    if state.scan.is_some() {
        return;
    }
    let copy_started = Instant::now();
    let owned = SaveRegistries {
        blocks: &content.blocks,
        fluids: &content.fluids,
        tools: &content.tools,
        creatures: &content.creatures,
        dimensions: &content.dimensions,
        cycles: &content.cycles,
    }
    .owned_for_pruning();
    info!("Saved-world catalog definitions copied on main thread: {:?}", copy_started.elapsed());
    let result: WorldScanResult = Arc::new(Mutex::new(None));
    let worker_result = Arc::clone(&result);
    match thread::Builder::new()
        .name("asteria-world-scan".to_owned())
        .spawn(move || {
            let scan_started = Instant::now();
            let verified = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                list_verified_worlds(&owned)
            }))
            .unwrap_or_else(|_| Err(io::Error::other("saved-world verification worker panicked")));
            info!(
                "Saved-world catalog verification: duration={:?}, successful={}, worlds={}",
                scan_started.elapsed(),
                verified.is_ok(),
                verified.as_ref().map_or(0, Vec::len)
            );
            if let Ok(mut slot) = worker_result.lock() {
                *slot = Some(verified);
            }
        })
    {
        Ok(_) => state.scan = Some(result),
        Err(error) => {
            state.error = format!(
                "{}: {error}",
                localization.text(language.get(), "worldSelection.scanStartError")
            );
        }
    }
}

fn poll_world_scan(
    mut commands: Commands,
    mut state: ResMut<WorldSelectionState>,
    list: Query<Entity, With<WorldListContainer>>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    let Some(scan) = state.scan.as_ref().cloned() else {
        return;
    };
    let result = scan.try_lock().ok().and_then(|mut slot| slot.take());
    let Some(result) = result else {
        return;
    };
    state.scan = None;
    match result {
        Ok(worlds) => {
            state.worlds = worlds;
            for list_entity in &list {
                commands.entity(list_entity).with_children(|parent| {
                    for world in &state.worlds {
                        spawn_world_entry(parent, world, &localization, language.get());
                    }
                });
            }
        }
        Err(error) => {
            state.error = format!(
                "{}: {error}",
                localization.text(language.get(), "worldSelection.scanError")
            );
        }
    }
}

fn spawn_world_entry(
    parent: &mut ChildSpawnerCommands,
    world: &WorldSummary,
    localization: &UiLocalization,
    language: crate::localization::Language,
) {
    let id = world.id.clone();
    parent
        .spawn((
            WorldListEntry(id.clone()),
            Node {
                width: percent(100),
                min_height: px(68),
                padding: UiRect::all(px(10)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                border: UiRect::all(px(2)),
                ..default()
            },
            BackgroundColor(theme::SURFACE_INSET),
            BorderColor::all(theme::BORDER),
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                },
                children![
                    typography::setting_title(world.id.clone()),
                    typography::caption(format_save_time(world.last_saved_unix_ms)),
                ],
            ));
            row.spawn(button(
                localization.text(language, "worldSelection.load").to_owned(),
                WorldSelectionAction::Load(id.clone()),
                px(160),
                COMPACT_CONTROL_HEIGHT,
                ButtonVariant::Primary,
            ));
            row.spawn(button(
                localization.text(language, "worldSelection.delete").to_owned(),
                WorldSelectionAction::Delete(id),
                px(160),
                COMPACT_CONTROL_HEIGHT,
                ButtonVariant::Danger,
            ));
        });
}

fn spawn_world_selection(
    mut commands: Commands,
    state: Res<WorldSelectionState>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    commands.spawn((
        Camera2d,
        BoxShadowSamples(8),
        DespawnOnExit(GameState::WorldSelection),
    ));
    let mut root = commands.spawn((
        DespawnOnExit(GameState::WorldSelection),
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(0),
            bottom: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(theme::SCREEN_BACKGROUND),
        theme::cosmic_background_gradient(),
    ));

    root.with_children(|root| {
        for &spec in STAR_FIELD {
            root.spawn(cosmic_background::star(spec));
        }

        root.spawn(screen::header()).with_children(|header| {
            header.spawn(typography::title(
                localization
                    .text(language.get(), "starting.loadWorlds")
                    .to_owned(),
            ));
        });

        root.spawn(screen::body()).with_children(|body| {
            body.spawn(screen::content_column(12.0))
                .with_children(|content| {
                    content
                        .spawn(surface::settings_content())
                        .with_children(|panel| {
                            panel
                                .spawn(Node {
                                    width: percent(100),
                                    height: percent(100),
                                    min_height: px(0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Stretch,
                                    row_gap: px(12),
                                    ..default()
                                })
                                .with_children(|column| {
                                    column.spawn((
                                        WorldListStatus,
                                        typography::caption(if state.scan.is_some() {
                                            localization
                                                .text(
                                                    language.get(),
                                                    "worldSelection.verifying",
                                                )
                                                .to_owned()
                                        } else {
                                            localization
                                                .text(
                                                    language.get(),
                                                    "worldSelection.noRestorable",
                                                )
                                                .to_owned()
                                        }),
                                    ));

                                    column
                                        .spawn(Node {
                                            display: Display::Grid,
                                            flex_grow: 1.0,
                                            min_height: px(0),
                                            width: percent(100),
                                            grid_template_columns: vec![
                                                RepeatedGridTrack::flex(1, 1.0),
                                                RepeatedGridTrack::auto(1),
                                            ],
                                            ..default()
                                        })
                                        .with_children(|frame| {
                                            let list_id = frame
                                                .spawn((
                                                    WorldListContainer,
                                                    ScrollPosition(Vec2::ZERO),
                                                    ScrollArea,
                                                    Node {
                                                        width: percent(100),
                                                        height: percent(100),
                                                        min_height: px(0),
                                                        padding: UiRect::right(px(12)),
                                                        flex_direction: FlexDirection::Column,
                                                        align_items: AlignItems::Stretch,
                                                        row_gap: px(10),
                                                        overflow: Overflow::scroll_y(),
                                                        ..default()
                                                    },
                                                ))
                                                .id();
                                            frame.spawn(scrollbar::vertical_scrollbar(list_id));
                                        });

                                    column.spawn((
                                        SelectionError,
                                        typography::caption(state.error.clone()),
                                        Node {
                                            display: if state.error.is_empty() {
                                                Display::None
                                            } else {
                                                Display::Flex
                                            },
                                            ..default()
                                        },
                                    ));
                                });
                        });
                });
        });

        root.spawn(screen::footer()).with_children(|footer| {
            footer.spawn(button(
                localization.text(language.get(), "newWorld.return").to_owned(),
                WorldSelectionAction::Back,
                px(360),
                COMPACT_CONTROL_HEIGHT,
                ButtonVariant::Normal,
            ));
        });
    });
}

#[derive(SystemParam)]
struct WorldSelectionLoadContext<'w> {
    content: WorldSelectionScanContent<'w>,
    inventory: ResMut<'w, PlayerHotbar>,
    save: ResMut<'w, InMemoryWorldSave>,
}

/// Polling a completed result and activating a world is cheap relative to
/// parsing and rehydrating every saved chunk, which happens on the worker.
fn poll_world_load(
    mut commands: Commands,
    mut state: ResMut<WorldSelectionState>,
    mut context: WorldSelectionLoadContext,
    mut transition: ResMut<ScreenTransition>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    if transition.is_active() {
        return;
    }
    let Some(pending) = state.loading.as_ref() else {
        return;
    };
    let (complete, abandoned, result) = match pending.result.try_lock() {
        Ok(mut slot) => (slot.complete, slot.abandoned, slot.result.take()),
        Err(_) => return,
    };
    if !complete {
        return;
    }
    let pending = state.loading.take().expect("completed load must be tracked");
    if abandoned {
        return;
    }
    let Some(result) = result else {
        state.error = format!(
            "{} {}: completed worker returned no result",
            localization.text(language.get(), "worldSelection.loadError"),
            pending.id
        );
        return;
    };
    let (mut snapshot, world, session_lock) = match result {
        Ok(loaded) => loaded,
        Err(error) => {
            state.error = format!(
                "{} {}: {error}",
                localization.text(language.get(), "worldSelection.loadError"),
                pending.id
            );
            return;
        }
    };
    // Rebuild scheduled runtime fluid work before mutating any live resources.
    // Saved ticks use textual fluid IDs, so a content change cannot silently
    // reinterpret a runtime FluidId.
    let pending_fluid_updates =
        match PendingFluidUpdates::from_saved(&snapshot.fluid_updates, &context.content.fluids) {
            Ok(pending) => pending,
            Err(error) => {
                state.error = format!(
                    "{} {}: {error}",
                    localization.text(language.get(), "worldSelection.loadError"),
                    pending.id
                );
                return;
            }
        };

    // Content or files might have changed since the catalog scan. The worker
    // revalidated its own immutable content; the live inventory is changed
    // only once that result has been accepted on the Bevy thread.
    if let Err(error) = context.inventory.restore_items(
        &snapshot.inventory,
        &context.content.blocks,
        &context.content.tools,
    ) {
        state.error = format!(
            "{}: {error}",
            localization.text(language.get(), "worldSelection.inventoryError")
        );
        return;
    }
    let mut rules = GameRules::default();
    rules.set_ticks_per_second(snapshot.ticks_per_second);
    context.save.begin_new_world(
        WorldSeed(snapshot.seed),
        &snapshot.dimension_id,
        rules,
        snapshot.spawn_biome.as_deref(),
        snapshot.biome_size_multiplier,
    );
    if let Some(player) = snapshot.player {
        context.save.save_player_state_with_health(
            LOCAL_PLAYER_ID,
            Vec3::from_array(player.position),
            if player.creative { GameMode::Creative } else { GameMode::Survival },
            player.health,
        );
    }
    commands.insert_resource(session_lock);
    commands.insert_resource(pending_fluid_updates);
    commands.insert_resource(PendingCreatureRestores::new(std::mem::take(
        &mut snapshot.creatures,
    )));
    commands.insert_resource(WorldSeed(snapshot.seed));
    commands.insert_resource(CurrentDimension { id: snapshot.dimension_id });
    commands.insert_resource(rules);
    commands.insert_resource(world);
    commands.insert_resource(WorldSession::loaded(pending.id, snapshot.day, snapshot.tick_in_day));
    commands.insert_resource(WorldLoadMode::Load);
    transition.request(ScreenTransitionTarget::game(GameState::Loading));
}

fn abandon_world_load(state: Res<WorldSelectionState>) {
    if let Some(pending) = state.loading.as_ref() {
        pending.abandon();
    }
}

#[derive(SystemParam)]
struct WorldSelectionActionContext<'w, 's> {
    content: WorldSelectionScanContent<'w>,
    entries: Query<'w, 's, (Entity, &'static WorldListEntry)>,
    localization: Res<'w, UiLocalization>,
    language: Res<'w, ActiveLanguage>,
}

fn handle_world_selection(
    mut commands: Commands,
    interactions: Query<(&Interaction, &WorldSelectionAction), Changed<Interaction>>,
    mut state: ResMut<WorldSelectionState>,
    mut transition: ResMut<ScreenTransition>,
    context: WorldSelectionActionContext,
) {
    if transition.is_active() {
        return;
    }

    // Back takes precedence even if another action changed in the same frame.
    if interactions.iter().any(|(interaction, action)| {
        *interaction == Interaction::Pressed && matches!(action, WorldSelectionAction::Back)
    }) {
        if let Some(pending) = state.loading.as_ref() {
            pending.abandon();
        }
        transition.request(ScreenTransitionTarget::game(GameState::StartingScreen));
        return;
    }

    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if state.loading.is_some() {
            state.error = context.localization
                .text(context.language.get(), "worldSelection.stillLoading")
                .to_owned();
            return;
        }

        match action {
            WorldSelectionAction::Delete(id) => {
                match delete_world(id) {
                    Ok(()) => {
                        state.worlds.retain(|world| world.id != *id);
                        state.error.clear();
                        if let Some((entity, _)) =
                            context.entries.iter().find(|(_, entry)| entry.0 == *id)
                        {
                            commands.entity(entity).despawn();
                        }
                    }
                    Err(error) => {
                        state.error = format!(
                            "{} {id}: {error}",
                            context.localization.text(context.language.get(), "worldSelection.deleteError")
                        );
                    }
                }
                return;
            }
            WorldSelectionAction::Load(id) => {
                if state.scan.is_some() {
                    state.error = context.localization
                        .text(context.language.get(), "worldSelection.stillVerifying")
                        .to_owned();
                    return;
                }

                let id = id.clone();
                let copy_started = Instant::now();
                let owned = context.content.owned_for_loading();
                info!(
                    "World {id} load definitions copied on main thread: {:?}",
                    copy_started.elapsed()
                );
                let result: WorldLoadResult = Arc::new(Mutex::new(WorldLoadSlot::default()));
                let worker_result = Arc::clone(&result);
                let worker_id = id.clone();
                match thread::Builder::new()
                    .name("asteria-world-load".to_owned())
                    .spawn(move || {
                        let load_started = Instant::now();
                        let loaded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            load_world(&worker_id, owned.registries())
                        }))
                        .unwrap_or_else(|_| {
                            Err(io::Error::other("saved-world loading worker panicked"))
                        });
                        info!(
                            "World {worker_id} load worker: duration={:?}, successful={}",
                            load_started.elapsed(),
                            loaded.is_ok()
                        );
                        let mut loaded = Some(loaded);
                        {
                            let mut slot = worker_result
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            slot.complete = true;
                            if !slot.abandoned {
                                slot.result = loaded.take();
                            }
                        }
                        drop(loaded);
                    })
                {
                    Ok(_) => {
                        state.loading = Some(PendingWorldLoad { id, result });
                        state.error = context.localization
                            .text(context.language.get(), "worldSelection.loadingSelected")
                            .to_owned();
                    }
                    Err(error) => {
                        state.error = format!(
                            "{}: {error}",
                            context.localization.text(context.language.get(), "worldSelection.loadStartError")
                        );
                    }
                }
                return;
            }
            WorldSelectionAction::Back => unreachable!("Back was handled above"),
        }
    }
}

type WorldListStatusQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static mut Node),
    (With<WorldListStatus>, Without<SelectionError>),
>;

type WorldListErrorQuery<'w, 's> =
    Query<'w, 's, (&'static mut Text, &'static mut Node), With<SelectionError>>;

fn sync_world_selection_feedback(
    state: Res<WorldSelectionState>,
    mut statuses: WorldListStatusQuery,
    mut errors: WorldListErrorQuery,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    if !state.is_changed() && !localization.is_changed() && !language.is_changed() {
        return;
    }

    let status = if state.scan.is_some() {
        localization
            .text(language.get(), "worldSelection.verifying")
            .to_owned()
    } else if state.worlds.is_empty() {
        localization
            .text(language.get(), "worldSelection.noRestorable")
            .to_owned()
    } else {
        String::new()
    };
    for (mut text, mut node) in &mut statuses {
        if text.0 != status {
            text.0 = status.clone();
        }
        let display = if status.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (mut text, mut node) in &mut errors {
        if text.0 != state.error {
            text.0.clone_from(&state.error);
        }
        let display = if state.error.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
    }
}

// Portable UTC rendering without relying on local timezone configuration.
fn format_save_time(unix_ms: u64) -> String {
    let seconds = unix_ms / 1_000;
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let seconds_in_day = seconds % 86_400;
    let z = days.saturating_add(719_468);
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    let hour = seconds_in_day / 3_600;
    let minute = seconds_in_day / 60 % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02} UTC")
}
