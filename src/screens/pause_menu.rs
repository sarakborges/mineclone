use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    localization::{ActiveLanguage, UiLocalization},
    ui::{
        button::menu_button,
        surface, theme,
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
    Settings,
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
            BackgroundColor(theme::OVERLAY),
        ))
        .with_children(|root| {
            root.spawn(surface::modal_panel()).with_children(|panel| {
                panel.spawn((
                    typography::title(localization.text(language, "pause.title").to_owned()),
                    Node {
                        margin: UiRect::bottom(px(10)),
                        ..default()
                    },
                ));
                panel.spawn(menu_button(
                    localization.text(language, "pause.resume").to_owned(),
                    PauseMenuAction::Resume,
                ));
                panel.spawn(menu_button(
                    localization.text(language, "common.settings").to_owned(),
                    PauseMenuAction::Settings,
                ));
                panel.spawn(menu_button(
                    localization.text(language, "pause.leaveWorld").to_owned(),
                    PauseMenuAction::LeaveWorld,
                ));
                panel.spawn(menu_button(
                    localization.text(language, "common.exitGame").to_owned(),
                    PauseMenuAction::ExitGame,
                ));
                panel.spawn((PauseSaveFeedback, typography::caption(String::new())));
            });
        });
}

fn handle_pause_menu_buttons(
    interactions: Query<(&Interaction, &PauseMenuAction), Changed<Interaction>>,
    snapshot: WorldSaveContext,
    mut session: ResMut<WorldSession>,
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
            PauseMenuAction::Settings => {
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
