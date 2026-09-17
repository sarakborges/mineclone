use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, settings_state::SettingsState, version},
    localization::{ActiveLanguage, UiLocalization},
    ui::{
        button::menu_button,
        cosmic_background::{self, STAR_FIELD},
        theme,
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
    },
};

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

fn setup_starting_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    commands.spawn((
        Camera2d,
        BoxShadowSamples(8),
        DespawnOnExit(GameState::StartingScreen),
    ));

    let logo = asset_server.load("branding/asteria_logo.png");
    let language = language.get();

    commands
        .spawn((
            DespawnOnExit(GameState::StartingScreen),
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(theme::SCREEN_BACKGROUND),
            theme::cosmic_background_gradient(),
        ))
        .with_children(|parent| {
            for &spec in STAR_FIELD {
                parent.spawn(cosmic_background::star(spec));
            }

            // The brand has its own visual space; actions share a borderless
            // surface rather than floating as four unrelated central buttons.
            parent
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    padding: UiRect::all(px(24)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    column_gap: px(24),
                    ..default()
                })
                .with_children(|content| {
                    content
                        .spawn(Node {
                            width: percent(50),
                            min_width: px(0),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            row_gap: px(22),
                            ..default()
                        })
                        .with_children(|brand| {
                            brand.spawn((
                                ImageNode::new(logo),
                                Node {
                                    width: percent(100),
                                    max_width: px(560),
                                    ..default()
                                },
                            ));
                            brand.spawn((
                                Node {
                                    width: px(84),
                                    height: px(4),
                                    border_radius: BorderRadius::all(px(2)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.40, 0.78, 0.94)),
                                BoxShadow(vec![ShadowStyle {
                                    color: theme::CYAN_GLOW,
                                    x_offset: px(0),
                                    y_offset: px(0),
                                    spread_radius: px(0),
                                    blur_radius: px(16),
                                }]),
                            ));
                        });

                    content
                        .spawn((
                            Node {
                                width: percent(43),
                                max_width: px(440),
                                min_width: px(0),
                                padding: UiRect::all(px(26)),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Stretch,
                                row_gap: px(14),
                                border_radius: BorderRadius::all(px(16)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.045, 0.032, 0.095, 0.94)),
                            theme::frosted_surface_gradient(),
                            BoxShadow(vec![ShadowStyle {
                                color: Color::srgba(0.0, 0.0, 0.0, 0.40),
                                x_offset: px(0),
                                y_offset: px(14),
                                spread_radius: px(0),
                                blur_radius: px(36),
                            }]),
                        ))
                        .with_children(|menu| {
                            menu.spawn(menu_button(
                                localization.text(language, "starting.newWorld").to_owned(),
                                StartingScreenAction::NewWorld,
                            ));
                            menu.spawn(menu_button(
                                localization.text(language, "starting.loadWorlds").to_owned(),
                                StartingScreenAction::LoadWorlds,
                            ));
                            menu.spawn(menu_button(
                                localization.text(language, "common.settings").to_owned(),
                                StartingScreenAction::Settings,
                            ));
                            menu.spawn(menu_button(
                                localization.text(language, "common.exitGame").to_owned(),
                                StartingScreenAction::ExitGame,
                            ));
                        });
                });

            parent.spawn((
                typography::caption(format!("v{}", version::VERSION.trim())),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(20),
                    bottom: px(16),
                    ..default()
                },
            ));
        });
}

fn handle_menu_buttons(
    interactions: Query<(&Interaction, &StartingScreenAction), Changed<Interaction>>,
    mut transition: ResMut<ScreenTransition>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            StartingScreenAction::NewWorld => {
                transition.request(ScreenTransitionTarget::game(GameState::NewWorld));
            }
            StartingScreenAction::LoadWorlds => {
                transition.request(ScreenTransitionTarget::game(GameState::WorldSelection));
            }
            StartingScreenAction::Settings => {
                transition.request(ScreenTransitionTarget::settings(SettingsState::Open));
            }
            StartingScreenAction::ExitGame => {
                app_exit.write(AppExit::Success);
            }
        }
    }
}
