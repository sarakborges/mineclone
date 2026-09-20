mod activation;
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
    localization::{ActiveLanguage, UiLocalization},
    player::hotbar::PlayerHotbar,
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
    world::{
        InMemoryWorldSave,
        save_catalog::{SaveRegistries, WorldSummary, delete_world},
    },
};

use self::{
    activation::{PreparedWorldActivation, WorldActivationError},
    layout::{
    SelectionError, WorldListContainer, WorldListEntry, WorldListStatus, spawn_world_entry,
        spawn_world_selection,
    },
    tasks::{PendingWorldLoad, PendingWorldScan},
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
    scan: Option<PendingWorldScan>,
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

/// Polling a completed result is cheap relative to parsing and rehydrating
/// saved chunks, which happens on the worker. All fallible activation
/// preparation finishes before any live world resource is mutated.
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
    if completion.abandoned {
        return;
    }
    let Some(result) = completion.result else {
        state.error = format!(
            "{} {}: completed worker returned no result",
            localization.text(language.get(), "worldSelection.loadError"),
            id
        );
        return;
    };
    let (snapshot, world, session_lock) = match result {
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

    let activation = match PreparedWorldActivation::prepare(
        id.clone(),
        snapshot,
        world,
        session_lock,
        &context.content.blocks,
        &context.content.fluids,
        &context.content.tools,
    ) {
        Ok(activation) => activation,
        Err(WorldActivationError::Load(error)) => {
            state.error = format!(
                "{} {}: {error}",
                localization.text(language.get(), "worldSelection.loadError"),
                id
            );
            return;
        }
        Err(WorldActivationError::Inventory(error)) => {
            state.error = format!(
                "{}: {error}",
                localization.text(language.get(), "worldSelection.inventoryError")
            );
            return;
        }
    };

    activation.commit(
        &mut commands,
        &mut context.inventory,
        &mut context.save,
    );
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
