use std::{
    io,
    sync::{Arc, Mutex},
    thread,
};

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        block::BlockRegistry, day_night_cycle::DayNightCycleRegistry,
        dimension::DimensionRegistry, fluid::FluidRegistry, tool::ToolRegistry,
    },
    localization::{ActiveLanguage, UiLocalization},
    player::{game_mode::GameMode, hotbar::PlayerHotbar, player_id::LOCAL_PLAYER_ID},
    ui::{
        button::menu_button, surface, theme,
        transition::{ScreenTransition, ScreenTransitionTarget}, typography,
    },
    world::{
        InMemoryWorldSave, WorldLoadMode, WorldSeed,
        dimension::CurrentDimension, game_rules::GameRules,
        save_catalog::{SaveRegistries, WorldSummary, list_verified_worlds, load_world},
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
            .add_systems(
                Update,
                (poll_world_scan, handle_world_selection, sync_world_selection_feedback)
                    .chain()
                    .run_if(in_state(GameState::WorldSelection)),
            );
    }
}

// Only the worker writes the result; Bevy polls without blocking the main thread.
type WorldScanResult = Arc<Mutex<Option<io::Result<Vec<WorldSummary>>>>>;

#[derive(Resource, Default)]
struct WorldSelectionState {
    worlds: Vec<WorldSummary>,
    selected: Option<String>,
    error: String,
    loading: bool,
    scan: Option<WorldScanResult>,
}

#[derive(Component, Clone)]
enum WorldSelectionAction {
    Select(String),
    Load,
    Back,
}

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

fn refresh_world_list(
    mut state: ResMut<WorldSelectionState>,
    content: WorldSelectionScanContent,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    state.selected = None;
    state.worlds.clear();
    state.error.clear();
    state.loading = true;
    // Returning to this menu while a prior scan is running reuses that scan
    // rather than creating an unbounded number of detached workers.
    if state.scan.is_some() {
        return;
    }
    let owned = SaveRegistries {
        blocks: &content.blocks,
        fluids: &content.fluids,
        tools: &content.tools,
        dimensions: &content.dimensions,
        cycles: &content.cycles,
    }
    .owned_for_pruning();
    let result: WorldScanResult = Arc::new(Mutex::new(None));
    let worker_result = Arc::clone(&result);
    match thread::Builder::new()
        .name("asteria-world-scan".to_owned())
        .spawn(move || {
            let verified = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                list_verified_worlds(&owned)
            }))
            .unwrap_or_else(|_| Err(io::Error::other("saved-world verification worker panicked")));
            if let Ok(mut slot) = worker_result.lock() {
                *slot = Some(verified);
            }
        })
    {
        Ok(_) => state.scan = Some(result),
        Err(error) => {
            state.loading = false;
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
    state.loading = false;
    match result {
        Ok(worlds) => {
            state.worlds = worlds;
            for list_entity in &list {
                commands.entity(list_entity).with_children(|parent| {
                    for world in &state.worlds {
                        parent.spawn(menu_button(
                            format!(
                                "{} — {}",
                                world.id,
                                format_save_time(world.last_saved_unix_ms)
                            ),
                            WorldSelectionAction::Select(world.id.clone()),
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
    commands
        .spawn((
            DespawnOnExit(GameState::WorldSelection),
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(theme::SCREEN_BACKGROUND),
            theme::cosmic_background_gradient(),
        ))
        .with_children(|root| {
            root.spawn(surface::modal_panel()).with_children(|panel| {
                panel.spawn(typography::title(
                    localization.text(language.get(), "starting.loadWorlds").to_owned(),
                ));
                panel.spawn((
                    WorldListStatus,
                    typography::caption(if state.loading {
                        localization.text(language.get(), "worldSelection.verifying").to_owned()
                    } else {
                        localization.text(language.get(), "worldSelection.noRestorable").to_owned()
                    }),
                ));
                panel.spawn((
                    WorldListContainer,
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                ));
                panel.spawn((SelectionFeedback, typography::caption(String::new())));
                panel.spawn((SelectionError, typography::caption(state.error.clone())));
                panel.spawn(menu_button(
                    localization.text(language.get(), "worldSelection.load").to_owned(),
                    WorldSelectionAction::Load,
                ));
                panel.spawn(menu_button(
                    localization.text(language.get(), "newWorld.return").to_owned(),
                    WorldSelectionAction::Back,
                ));
            });
        });
}

#[derive(SystemParam)]
struct WorldSelectionLoadContext<'w> {
    blocks: Res<'w, BlockRegistry>,
    fluids: Res<'w, FluidRegistry>,
    tools: Res<'w, ToolRegistry>,
    dimensions: Res<'w, DimensionRegistry>,
    cycles: Res<'w, DayNightCycleRegistry>,
    inventory: ResMut<'w, PlayerHotbar>,
    save: ResMut<'w, InMemoryWorldSave>,
}

fn handle_world_selection(
    mut commands: Commands,
    interactions: Query<(&Interaction, &WorldSelectionAction), Changed<Interaction>>,
    mut state: ResMut<WorldSelectionState>,
    mut context: WorldSelectionLoadContext,
    mut transition: ResMut<ScreenTransition>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    if transition.is_active() {
        return;
    }
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            WorldSelectionAction::Select(id) => {
                state.selected = Some(id.clone());
                state.error.clear();
            }
            WorldSelectionAction::Back => {
                transition.request(ScreenTransitionTarget::game(GameState::StartingScreen));
                return;
            }
            WorldSelectionAction::Load => {
                if state.loading {
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
                let registries = SaveRegistries {
                    blocks: &context.blocks,
                    fluids: &context.fluids,
                    tools: &context.tools,
                    dimensions: &context.dimensions,
                    cycles: &context.cycles,
                };
                let (snapshot, world) = match load_world(&id, registries) {
                    Ok(loaded) => loaded,
                    Err(error) => {
                        state.error = format!(
                            "{} {id}: {error}",
                            localization.text(language.get(), "worldSelection.loadError")
                        );
                        return;
                    }
                };
                // Revalidate on load: files could change after the background
                // scan displayed its timestamp.
                if let Err(error) = context.inventory.restore_items(
                    &snapshot.inventory,
                    &context.blocks,
                    &context.tools,
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
                commands.insert_resource(WorldSession::loaded(id, snapshot.day, snapshot.tick_in_day));
                commands.insert_resource(WorldLoadMode::Load);
                transition.request(ScreenTransitionTarget::game(GameState::Loading));
                return;
            }
        }
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
