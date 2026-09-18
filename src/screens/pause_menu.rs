use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::{SettingsScreenMode, SettingsState}},
    localization::{ActiveLanguage, UiLocalization},
    ui::{
        button::{menu_button, standard_button},
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
        visibility::set_visibility,
    },
    world::save_session::{WorldSaveContext, WorldSession},
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
                menu.spawn(menu_button(
                    localization.text(language, "pause.resume").to_owned(),
                    PauseMenuAction::Resume,
                ));
                menu.spawn(menu_button(
                    localization.text(language, "pause.leaveWorld").to_owned(),
                    PauseMenuAction::LeaveWorld,
                ));
                menu.spawn((
                    Node {
                        width: px(360),
                        flex_direction: FlexDirection::Row,
                        column_gap: px(12),
                        ..default()
                    ),
                    children![
                        standard_button(
                            localization.text(language, "settings.section.worldSettings").to_owned(),
                            PauseMenuAction::WorldSettings,
                            174.0,
                            crate::ui::button::ButtonVariant::Normal,
                        ),
                        standard_button(
                            localization.text(language, "common.gameSettings").to_owned(),
                            PauseMenuAction::GameSettings,
                            174.0,
                            crate::ui::button::ButtonVariant::Normal,
                        ),
                    ],
                ));
                menu.spawn(menu_button(
                    localization.text(language, "common.exitGame").to_owned(),
                    PauseMenuAction::ExitGame,
                ));
                menu.spawn((PauseSaveFeedback, typography::caption(String::new())));
            });
        });
}

fn handle_pause_menu_buttons(
    interactions: Query<(&Interaction, &PauseMenuAction), Changed<Interaction>>,
    snapshot: WorldSaveContext,
    mut session: ResMut<WorldSession>,
    mut settings_mode: ResMut<SettingsScreenMode>,
    mut feedback: Query<&mut Text, With<PauseSaveFeedback>>,
    mut transition: ResMut<ScreenTransition>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if transition.is_active() {
        return;
    }
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            PauseMenuAction::Resume => {
                transition.request(ScreenTransitionTarget::pause(PauseState::Running));
            }
            PauseMenuAction::WorldSettings => {
                *settings_mode = SettingsScreenMode::World;
                transition.request(ScreenTransitionTarget::settings(SettingsState::Open));
            }
            PauseMenuAction::GameSettings => {
                *settings_mode = SettingsScreenMode::Game;
                transition.request(ScreenTransitionTarget::settings(SettingsState::Open));
            }
            PauseMenuAction::LeaveWorld | PauseMenuAction::ExitGame => {
                if let Err(error) = session.persist(&snapshot) {
                    error!("World save failed; keeping current world loaded: {error}");
                    if let Ok(mut label) = feedback.single_mut() {
                        label.0 = format!("Save failed: {error}. World kept open.");
                    }
                    return;
                }
                if matches!(action, PauseMenuAction::LeaveWorld) {
                    transition.request(
                        ScreenTransitionTarget::game(GameState::StartingScreen)
                            .with_pause(PauseState::Running),
                    );
                } else {
                    app_exit.write(AppExit::Success);
                }
                return;
            }
        }
    }
}
