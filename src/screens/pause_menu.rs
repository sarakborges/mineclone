use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    player::{camera::GameplayCamera, game_mode::GameMode, player_id::PlayerId},
    ui::{
        button::menu_button,
        surface, theme,
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
    },
    world::{InMemoryWorldSave, game_rules::GameRules},
};

pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(PauseState::Paused), spawn_pause_menu)
            .add_systems(OnEnter(SettingsState::Open), hide_pause_menu)
            .add_systems(
                OnEnter(SettingsState::Closed),
                show_pause_menu.run_if(in_state(PauseState::Paused)),
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

#[derive(Component, Clone, Copy)]
enum PauseMenuAction {
    Resume,
    Settings,
    LeaveWorld,
    ExitGame,
}

fn spawn_pause_menu(mut commands: Commands) {
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
                    typography::title("PAUSED"),
                    Node {
                        margin: UiRect::bottom(px(10)),
                        ..default()
                    },
                ));
                panel.spawn(menu_button("Resume", PauseMenuAction::Resume));
                panel.spawn(menu_button("Settings", PauseMenuAction::Settings));
                panel.spawn(menu_button("Leave World", PauseMenuAction::LeaveWorld));
                panel.spawn(menu_button("Exit Game", PauseMenuAction::ExitGame));
            });
        });
}

fn hide_pause_menu(mut roots: Query<&mut Visibility, With<PauseMenuRoot>>) {
    for mut visibility in &mut roots {
        *visibility = Visibility::Hidden;
    }
}

fn show_pause_menu(mut roots: Query<&mut Visibility, With<PauseMenuRoot>>) {
    for mut visibility in &mut roots {
        *visibility = Visibility::Visible;
    }
}

fn handle_pause_menu_buttons(
    interactions: Query<(&Interaction, &PauseMenuAction), Changed<Interaction>>,
    player: Query<(&PlayerId, &Transform, &GameMode), With<GameplayCamera>>,
    game_rules: Res<GameRules>,
    mut save: ResMut<InMemoryWorldSave>,
    mut transition: ResMut<ScreenTransition>,
    mut app_exit: MessageWriter<AppExit>,
) {
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
            PauseMenuAction::LeaveWorld => {
                save.save_game_rules(*game_rules);
                if let Ok((player_id, transform, game_mode)) = player.single() {
                    save.save_player_state(*player_id, transform.translation, *game_mode);
                }

                transition.request(
                    ScreenTransitionTarget::game(GameState::StartingScreen)
                        .with_pause(PauseState::Running),
                );
            }
            PauseMenuAction::ExitGame => {
                app_exit.write(AppExit::Success);
            }
        }
    }
}
