use bevy::prelude::*;

use crate::app::{
    game_state::GameState,
    pause_state::PauseState,
    settings_state::SettingsState,
};

const OVERLAY_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.68);
const BUTTON_COLOR: Color = Color::srgb(0.12, 0.14, 0.18);
const BUTTON_HOVER_COLOR: Color = Color::srgb(0.18, 0.21, 0.27);
const BUTTON_PRESSED_COLOR: Color = Color::srgb(0.09, 0.11, 0.14);
const TEXT_COLOR: Color = Color::srgb(0.92, 0.94, 0.97);

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
    commands.spawn((
        DespawnOnExit(PauseState::Paused),
        PauseMenuRoot,
        Node {
            width: percent(100),
            height: percent(100),
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(16),
            ..default()
        },
        BackgroundColor(OVERLAY_COLOR),
        children![
            (
                Text::new("PAUSED"),
                TextFont {
                    font_size: FontSize::Px(48.0),
                    ..default()
                },
                TextColor(TEXT_COLOR),
                Node {
                    margin: UiRect::bottom(px(24)),
                    ..default()
                },
            ),
            pause_button("Resume", PauseMenuAction::Resume),
            pause_button("Settings", PauseMenuAction::Settings),
            pause_button("Leave World", PauseMenuAction::LeaveWorld),
            pause_button("Exit Game", PauseMenuAction::ExitGame),
        ],
    ));
}

fn pause_button(label: &'static str, action: PauseMenuAction) -> impl Bundle {
    (
        Button,
        action,
        Node {
            width: px(280),
            height: px(56),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(1)),
            ..default()
        },
        BackgroundColor(BUTTON_COLOR),
        BorderColor::all(Color::srgb(0.28, 0.32, 0.4)),
        children![(
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(TEXT_COLOR),
        )],
    )
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
    mut interactions: Query<
        (&Interaction, &PauseMenuAction, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut next_pause_state: ResMut<NextState<PauseState>>,
    mut next_settings_state: ResMut<NextState<SettingsState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, action, mut background) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                *background = BUTTON_PRESSED_COLOR.into();

                match action {
                    PauseMenuAction::Resume => next_pause_state.set(PauseState::Running),
                    PauseMenuAction::Settings => {
                        next_settings_state.set(SettingsState::Open);
                    }
                    PauseMenuAction::LeaveWorld => {
                        next_pause_state.set(PauseState::Running);
                        next_game_state.set(GameState::StartingScreen);
                    }
                    PauseMenuAction::ExitGame => {
                        app_exit.write(AppExit::Success);
                    }
                }
            }
            Interaction::Hovered => *background = BUTTON_HOVER_COLOR.into(),
            Interaction::None => *background = BUTTON_COLOR.into(),
        }
    }
}
