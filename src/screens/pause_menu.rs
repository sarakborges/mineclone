use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::{SettingsScreenMode, SettingsState}},
    player::camera::GameplayWorldCamera,
    localization::{ActiveLanguage, UiLocalization},
    ui::{
        button::{button, ButtonVariant, COMPACT_CONTROL_HEIGHT},
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
        visibility::set_visibility,
    },
    world::{
        save_session::{WorldSaveContext, WorldSession},
        thumbnail::{
            WorldThumbnailCapture, WorldThumbnailCompletion, begin_world_thumbnail_capture,
        },
    },
};

pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(PauseState::Paused), spawn_pause_menu)
            .add_systems(
                OnEnter(SettingsState::Open),
                set_visibility::<PauseMenuRoot, false>,
            )
            .add_systems(
                OnEnter(SettingsState::Closed),
                set_visibility::<PauseMenuRoot, true>.run_if(in_state(PauseState::Paused)),
            )
            .add_systems(
                Update,
                handle_pause_menu_buttons
                    .run_if(in_state(PauseState::Paused))
                    .run_if(in_state(SettingsState::Closed)),
            );
    }
}

#[derive(Component)]
struct PauseMenuRoot;

#[derive(Component)]
struct PauseSaveFeedback;

#[derive(Component, Clone, Copy)]
enum PauseMenuAction {
    Resume,
    WorldSettings,
    GameSettings,
    LeaveWorld,
    ExitGame,
}

fn spawn_pause_menu(
    mut commands: Commands,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    let language = language.get();

    commands
        .spawn((
            DespawnOnExit(PauseState::Paused),
            PauseMenuRoot,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.68)),
            GlobalZIndex(1000),
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: px(360),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(12),
                ..default()
            })
            .with_children(|menu| {
                menu.spawn(button(
                    localization.text(language, "pause.resume").to_owned(),
                    PauseMenuAction::Resume,
                    px(360),
                    COMPACT_CONTROL_HEIGHT,
                    ButtonVariant::Normal,
                ));
                menu.spawn((
                    Node {
                        width: px(360),
                        flex_direction: FlexDirection::Row,
                        column_gap: px(12),
                        ..default()
                    },
                    children![
                        button(
                            localization.text(language, "settings.section.worldSettings").to_owned(),
                            PauseMenuAction::WorldSettings,
                            px(174),
                            COMPACT_CONTROL_HEIGHT,
                            ButtonVariant::Normal,
                        ),
                        button(
                            localization.text(language, "common.gameSettings").to_owned(),
                            PauseMenuAction::GameSettings,
                            px(174),
                            COMPACT_CONTROL_HEIGHT,
                            ButtonVariant::Normal,
                        ),
                    ],
                ));
                menu.spawn(button(
                    localization.text(language, "pause.leaveWorld").to_owned(),
                    PauseMenuAction::LeaveWorld,
                    px(360),
                    COMPACT_CONTROL_HEIGHT,
                    ButtonVariant::Normal,
                ));
                menu.spawn(button(
                    localization.text(language, "common.exitGame").to_owned(),
                    PauseMenuAction::ExitGame,
                    px(360),
                    COMPACT_CONTROL_HEIGHT,
                    ButtonVariant::Danger,
                ));
                menu.spawn((PauseSaveFeedback, typography::caption(String::new())));
            });
        });
}

#[derive(SystemParam)]
struct PauseMenuContext<'w, 's> {
    session: Res<'w, WorldSession>,
    non_world_cameras: Query<
        'w,
        's,
        (Entity, &'static mut Camera),
        Without<GameplayWorldCamera>,
    >,
    thumbnail_captures: Query<'w, 's, (), With<WorldThumbnailCapture>>,
    settings_mode: ResMut<'w, SettingsScreenMode>,
    feedback: Query<'w, 's, &'static mut Text, With<PauseSaveFeedback>>,
    transition: ResMut<'w, ScreenTransition>,
    app_exit: MessageWriter<'w, AppExit>,
}

fn handle_pause_menu_buttons(
    mut commands: Commands,
    interactions: Query<(&Interaction, &PauseMenuAction), Changed<Interaction>>,
    snapshot: WorldSaveContext,
    mut context: PauseMenuContext,
) {
    if context.transition.is_active() || !context.thumbnail_captures.is_empty() {
        return;
    }
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            PauseMenuAction::Resume => {
                context.transition.request(ScreenTransitionTarget::pause(PauseState::Running));
            }
            PauseMenuAction::WorldSettings => {
                *context.settings_mode = SettingsScreenMode::World;
                context.transition.request(ScreenTransitionTarget::settings(SettingsState::Open));
            }
            PauseMenuAction::GameSettings => {
                *context.settings_mode = SettingsScreenMode::Game;
                context.transition.request(ScreenTransitionTarget::settings(SettingsState::Open));
            }
            PauseMenuAction::LeaveWorld | PauseMenuAction::ExitGame => {
                if let Err(error) = context.session.persist(&snapshot) {
                    error!("World save failed; keeping current world loaded: {error}");
                    if let Ok(mut label) = context.feedback.single_mut() {
                        label.0 = format!("Save failed: {error}. World kept open.");
                    }
                    return;
                }
                let completion = if matches!(action, PauseMenuAction::LeaveWorld) {
                    WorldThumbnailCompletion::LeaveWorld
                } else {
                    WorldThumbnailCompletion::ExitGame
                };
                let Some(world_id) = context.session.id() else {
                    error!("World was saved without an active world session id");
                    if matches!(action, PauseMenuAction::LeaveWorld) {
                        context.transition.request(
                            ScreenTransitionTarget::game(GameState::StartingScreen)
                                .with_pause(PauseState::Running),
                        );
                    } else {
                        context.app_exit.write(AppExit::Success);
                    }
                    return;
                };
                begin_world_thumbnail_capture(
                    &mut commands,
                    &mut context.non_world_cameras,
                    world_id,
                    completion,
                );
                return;
            }
        }
    }
}
