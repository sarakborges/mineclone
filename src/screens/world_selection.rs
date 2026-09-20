mod layout;
mod tasks;

use bevy::{ecs::system::SystemParam, prelude::*};

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
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
    world::{
        InMemoryWorldSave, WorldLoadMode, WorldSeed,
        dimension::CurrentDimension, game_rules::GameRules,
        fluid_updates::PendingFluidUpdates,
        save_catalog::{SaveRegistries, WorldSummary, delete_world},
        save_session::WorldSession,
    },
};

use self::layout::{
    SelectionError, WorldListContainer, WorldListEntry, WorldListStatus, spawn_world_entry,
    spawn_world_selection,
};
use self::tasks::{
    PendingWorldLoad, PendingWorldScan, WorldLoadCompletion,
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

#[derive(Resource, Default)]
struct WorldSelectionState {
    worlds: Vec<WorldSummary>,
    error: String,
    scan: Option<PendingWorldScan>
    loading: Option<PendingWorldLoad>,
}

#[derive(Component, Clone)]
enum WorldSelectionAction {
    Load(String),
    Delete(String),
    Back,
}

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

impl WorldSelectionScanContent<'_> {
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
    match PendingWorldScan::start(content.registries()) {
        Ok(scan) => state.scan = Some(scan),
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
    let Some(scan) = state.scan.as_ref() else {
        return;
    };
    let result = scan.poll();
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
    let Some(completion) = pending.poll() else {
        return;
    };
    let pending = state.loading.take().expect("completed load must be tracked");
    let id = pending.id().to_owned();
    let result = match completion {
        WorldLoadCompletion::Abandoned => return,
        WorldLoadCompletion::Finished(Some(result)) => result,
        WorldLoadCompletion::Finished(None) => {
            state.error = format!(
                "{} {}: completed worker returned no result",
                localization.text(language.get(), "worldSelection.loadError"),
                id
            );
            return;
        }
    };
    let (mut snapshot, world, session_lock) = match result {
        Ok(loaded) => loaded,
        Err(error) => {
            state.error = format!(
                "{} {}: {error}",
                localization.text(language.get(), "worldSelection.loadError"),
                id
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
                    id
                );
                return;
            }
        };

    // Content or files might have changed since the catalog scan. The worker
    // revalidated its own immutable content; the live inventory is changed
    // only once that result has been accepted on the Bevy thread.
    if let Err(error) = context.inventory.restore_items_and_selection(
        &snapshot.inventory,
        snapshot.selected_hotbar_slot,
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
            Some((player.yaw, player.pitch)),
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
    commands.insert_resource(WorldSession::loaded(id, snapshot.day, snapshot.tick_in_day));
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
                match PendingWorldLoad::start(id.clone(), context.content.registries()) {
                    Ok(pending) => {
                        state.loading = Some(pending);
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
