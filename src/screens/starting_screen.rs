use bevy::prelude::*;

use crate::app::{game_state::GameState, settings_state::SettingsState};

const BACKGROUND_COLOR: Color = Color::srgb(0.055, 0.065, 0.08);
const BUTTON_COLOR: Color = Color::srgb(0.12, 0.14, 0.18);
const BUTTON_HOVER_COLOR: Color = Color::srgb(0.18, 0.21, 0.27);
const BUTTON_PRESSED_COLOR: Color = Color::srgb(0.09, 0.11, 0.14);
const TEXT_COLOR: Color = Color::srgb(0.92, 0.94, 0.97);

pub struct StartingScreenPlugin;

impl Plugin for StartingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::StartingScreen), setup_starting_screen)
            .add_systems(
                Update,
                handle_menu_buttons
                    .run_if(in_state(GameState::StartingScreen))
                    .run_if(in_state(SettingsState::Closed)),
            );
    }
}

#[derive(Component, Clone, Copy)]
enum StartingScreenAction {
    NewWorld,
    LoadWorlds,
    Settings,
    ExitGame,
}

fn setup_starting_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, DespawnOnExit(GameState::StartingScreen)));

    let logo = asset_server.load("branding/asteria_logo.png");

    commands.spawn((
        DespawnOnExit(GameState::StartingScreen),
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(16),
            ..default()
        },
        BackgroundColor(BACKGROUND_COLOR),
        children![
            (
                ImageNode::new(logo),
                Node {
                    width: px(512),
                    margin: UiRect::bottom(px(24)),
                    ..default()
                },
            ),
            menu_button("New World", StartingScreenAction::NewWorld),
            menu_button("Load Worlds", StartingScreenAction::LoadWorlds),
            menu_button("Settings", StartingScreenAction::Settings),
            menu_button("Exit Game", StartingScreenAction::ExitGame),
        ],
    ));
}

fn menu_button(label: &'static str, action: StartingScreenAction) -> impl Bundle {
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

fn handle_menu_buttons(
    mut interactions: Query<
        (&Interaction, &StartingScreenAction, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_settings_state: ResMut<NextState<SettingsState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, action, mut background) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                *background = BUTTON_PRESSED_COLOR.into();

                match action {
                    StartingScreenAction::NewWorld => next_game_state.set(GameState::Loading),
                    StartingScreenAction::LoadWorlds => {}
                    StartingScreenAction::Settings => {
                        next_settings_state.set(SettingsState::Open);
                    }
                    StartingScreenAction::ExitGame => {
                        app_exit.write(AppExit::Success);
                    }
                }
            }
            Interaction::Hovered => *background = BUTTON_HOVER_COLOR.into(),
            Interaction::None => *background = BUTTON_COLOR.into(),
        }
    }
}
