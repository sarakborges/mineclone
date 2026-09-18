use std::{
    io,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use bevy::{ecs::system::SystemParam, log::warn, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        block::BlockRegistry, day_night_cycle::DayNightCycleRegistry,
        dimension::DimensionRegistry, fluid::FluidRegistry, tool::ToolRegistry,
    },
    localization::{ActiveLanguage, UiLocalization},
    player::{game_mode::GameMode, hotbar::PlayerHotbar, player_id::LOCAL_PLAYER_ID},
    ui::{
        button::{danger_button, menu_button, primary_button, standard_button_with_marker, ButtonVariant}, theme,
        transition::{ScreenTransition, ScreenTransitionTarget}, typography,
    },
    voxel::world::VoxelWorld,
    world::{
        InMemoryWorldSave, WorldLoadMode, WorldSeed,
        dimension::CurrentDimension, game_rules::GameRules,
        save_catalog::{SaveRegistries, WorldSnapshot, WorldSummary, delete_world, list_verified_worlds, load_world},
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
                (poll_world_scan, handle_world_selection, poll_world_load, sync_world_selection_entries, sync_world_selection_feedback)
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
    result: Option<io::Result<(WorldSnapshot, VoxelWorld)>>,
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
    selected: Option<String>,
    error: String,
    scan: Option<WorldScanResult>,
    loading: Option<PendingWorldLoad>,
}

#[derive(Component, Clone)]
enum WorldSelectionAction {
    Select(String),
    Load,
    Delete,
    Back,
}

#[derive(Component)]
struct WorldListEntry(String);

#[derive(Component)]
struct SelectionFeedback;

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
    dimensions: Res<'w, DimensionRegistry>,
    cycles: Res<'w, DayNightCycleRegistry>,
}

/// The selected-world loader owns its content definitions; no Bevy resource
/// borrows escape into a detached thread or survive a menu/world transition.
struct OwnedLoadContent {
    blocks: BlockRegistry,
    fluids: FluidRegistry,
    tools: ToolRegistry,
    dimensions: DimensionRegistry,
    cycles: DayNightCycleRegistry,
}

impl WorldSelectionScanContent<'_> {
    fn owned_for_loading(&self) -> OwnedLoadContent {
        let mut tools = ToolRegistry::default();
        for definition in self.tools.iter() {
            tools.insert(definition.clone());
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
    state.selected = None;
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
    mut statuses: Query<&mut Text, With<WorldListStatus>>,
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
                        let selected = state.selected.as_deref() == Some(world.id.as_str());
                        parent.spawn((
                            standard_button_with_marker(
                                format!("{} — {}", world.id, format_save_time(world.last_saved_unix_ms)),
                                WorldSelectionAction::Select(world.id.clone()),
                                0.0,
                                ButtonVariant::from_active(selected),
                                WorldListEntry(world.id.clone()),
                            ),
                        ));
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
    for mut status in &mut statuses {
        status.0 = if state.worlds.is_empty() {
            localization.text(language.get(), "worldSelection.noRestorable").to_owned()
        } else {
            String::new()
        };
    }
}

fn spawn_world_selection(
    mut commands: Commands,
    state: Res<WorldSelectionState>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    commands.spawn((Camera2d, DespawnOnExit(GameState::WorldSelection)));
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
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(theme::SCREEN_BACKGROUND),
        theme::cosmic_background_gradient(),
    ));

    root.with_children(|root| {
        root.spawn((
            Node {
                width: percent(100),
                height: px(116),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            children![typography::title(
                localization.text(language.get(), "starting.loadWorlds").to_owned(),
            )],
        ));

        root.spawn((
            Node {
                width: px(1120),
                max_width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                padding: UiRect::axes(px(18), px(32)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(12),
                ..default()
            },
            children![
                (
                    WorldListStatus,
                    typography::caption(if state.scan.is_some() {
                        localization.text(language.get(), "worldSelection.verifying").to_owned()
                    } else {
                        localization.text(language.get(), "worldSelection.noRestorable").to_owned()
                    }),
                ),
                (
                    Node {
                        flex_grow: 1.0,
                        min_height: px(0),
                        width: percent(100),
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    children![
                        (
                            WorldListContainer,
                            ScrollPosition(Vec2::ZERO),
                            Node {
                                width: percent(100),
                                height: percent(100),
                                min_height: px(0),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Stretch,
                                row_gap: px(8),
                                overflow: Overflow::scroll_y(),
                                padding: UiRect::all(px(10)),
                                border: UiRect::all(px(2)),
                                ..default()
                            },
                            BackgroundColor(theme::SURFACE_INSET),
                            BorderColor::all(theme::BORDER),
                        ),
                    ],
                ),
                (SelectionFeedback, typography::caption(String::new())),
                (SelectionError, typography::caption(state.error.clone())),
            ],
        ));

        root.spawn((
            Node {
                width: percent(100),
                height: px(104),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(18),
                ..default()
            },
            children![
                primary_menu_button(
                    localization.text(language.get(), "worldSelection.load").to_owned(),
                    WorldSelectionAction::Load,
                ),
                menu_button(
                    localization.text(language.get(), "newWorld.return").to_owned(),
                    WorldSelectionAction::Back,
                ),
                danger_button(
                    localization.text(language.get(), "worldSelection.delete").to_owned(),
                    WorldSelectionAction::Delete,
                ),
            ],
        ));
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
    let (snapshot, world) = match result {
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
    );
    if let Some(player) = snapshot.player {
        context.save.save_player_state(
            LOCAL_PLAYER_ID,
            Vec3::from_array(player.position),
            if player.creative { GameMode::Creative } else { GameMode::Survival },
        );
    }
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

fn handle_world_selection(
    interactions: Query<(&Interaction, &WorldSelectionAction), Changed<Interaction>>,
    mut state: ResMut<WorldSelectionState>,
    content: WorldSelectionScanContent,
    mut transition: ResMut<ScreenTransition>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    if transition.is_active() {
        return;
    }
    // Back takes precedence even if Load and Back both changed to Pressed in
    // one frame; do not depend on entity iteration order to cancel a load.
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
        // Never start a second loader while an existing or abandoned worker
        // still owns its result. Its completion will be consumed, not applied.
        if state.loading.is_some() {
            state.error = localization
                .text(language.get(), "worldSelection.stillLoading")
                .to_owned();
            return;
        }
        match action {
            WorldSelectionAction::Select(id) => {
                state.selected = Some(id.clone());
                state.error.clear();
            }
            WorldSelectionAction::Delete => {
                let Some(id) = state.selected.clone() else {
                    state.error = localization.text(language.get(), "worldSelection.selectFirst").to_owned();
                    return;
                };
                match delete_world(&id) {
                    Ok(()) => {
                        state.worlds.retain(|world| world.id != id);
                        state.selected = None;
                        state.error.clear();
                    }
                    Err(error) => {
                        state.error = format!(
                            "{} {id}: {error}",
                            localization.text(language.get(), "worldSelection.deleteError")
                        );
                    }
                }
                return;
            }
            WorldSelectionAction::Load => {
                if state.scan.is_some() {
                    state.error = localization
                        .text(language.get(), "worldSelection.stillVerifying")
                        .to_owned();
                    return;
                }
                let Some(id) = state.selected.clone() else {
                    state.error = localization
                        .text(language.get(), "worldSelection.selectFirst")
                        .to_owned();
                    return;
                };
                let copy_started = Instant::now();
                let owned = content.owned_for_loading();
                info!("World {id} load definitions copied on main thread: {:?}", copy_started.elapsed());
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
                        .unwrap_or_else(|_| Err(io::Error::other("saved-world loading worker panicked")));
                        info!(
                            "World {worker_id} load worker: duration={:?}, successful={}",
                            load_started.elapsed(),
                            loaded.is_ok()
                        );
                        let mut loaded = Some(loaded);
                        {
                            let mut slot = worker_result.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                            slot.complete = true;
                            if !slot.abandoned {
                                slot.result = loaded.take();
                            }
                        }
                        // If Back won, the original worker owns and drops its
                        // discarded VoxelWorld here, never on the Bevy frame.
                        drop(loaded);
                    })
                {
                    Ok(_) => {
                        state.loading = Some(PendingWorldLoad { id, result });
                        state.error = localization
                            .text(language.get(), "worldSelection.loadingSelected")
                            .to_owned();
                    }
                    Err(error) => {
                        state.error = format!(
                            "{}: {error}",
                            localization.text(language.get(), "worldSelection.loadStartError")
                        );
                    }
                }
                return;
            }
            WorldSelectionAction::Back => unreachable!("Back was handled above"),
        }
    }
}

fn sync_world_selection_entries(
    state: Res<WorldSelectionState>,
    mut entries: Query<(&WorldListEntry, &mut ButtonVariant)>,
) {
    if !state.is_changed() { return; }
    for (entry, mut variant) in &mut entries {
        *variant = ButtonVariant::from_active(state.selected.as_deref() == Some(entry.0.as_str()));
    }
}

fn sync_world_selection_feedback(
    state: Res<WorldSelectionState>,
    mut selected: Query<&mut Text, (With<SelectionFeedback>, Without<SelectionError>)>,
    mut errors: Query<&mut Text, With<SelectionError>>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    if !state.is_changed() {
        return;
    }
    for mut text in &mut selected {
        text.0 = state.selected.as_deref().map_or_else(String::new, |id| {
            format!("{}: {id}", localization.text(language.get(), "worldSelection.selected"))
        });
    }
    for mut text in &mut errors {
        text.0.clone_from(&state.error);
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
