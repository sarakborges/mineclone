use bevy::prelude::*;

use crate::app::{game_state::GameState, settings_state::SettingsState};

const BACKGROUND_COLOR: Color = Color::srgb(0.018, 0.014, 0.036);
const PANEL_COLOR: Color = Color::srgba(0.035, 0.027, 0.072, 0.82);
const BUTTON_COLOR: Color = Color::srgba(0.085, 0.068, 0.145, 0.88);
const BUTTON_HOVER_COLOR: Color = Color::srgba(0.205, 0.125, 0.38, 0.94);
const BUTTON_PRESSED_COLOR: Color = Color::srgba(0.125, 0.085, 0.235, 0.98);
const TEXT_COLOR: Color = Color::srgb(0.94, 0.95, 1.0);
const VERSION_COLOR: Color = Color::srgba(0.73, 0.75, 0.86, 0.58);
const VIOLET_GLOW: Color = Color::srgba(0.49, 0.27, 1.0, 0.22);
const CYAN_GLOW: Color = Color::srgba(0.23, 0.76, 1.0, 0.12);

const STAR_FIELD: &[(f32, f32, f32, f32, f32, f32, f32, f32, f32)] = &[
    (7.0, 12.0, 1.4, 0.1, 0.38, 0.88, 0.91, 1.0, 0.34),
    (15.0, 73.0, 1.8, 1.7, 0.31, 0.72, 0.61, 1.0, 0.26),
    (22.0, 28.0, 1.2, 2.4, 0.44, 0.67, 0.89, 1.0, 0.30),
    (29.0, 88.0, 1.4, 0.7, 0.35, 0.96, 0.92, 1.0, 0.22),
    (34.0, 16.0, 1.0, 3.2, 0.41, 0.72, 0.64, 1.0, 0.23),
    (41.0, 68.0, 1.5, 4.1, 0.30, 0.90, 0.95, 1.0, 0.25),
    (47.0, 9.0, 1.2, 1.1, 0.43, 0.64, 0.86, 1.0, 0.25),
    (53.0, 82.0, 1.4, 2.9, 0.33, 0.82, 0.66, 1.0, 0.24),
    (59.0, 21.0, 1.0, 0.5, 0.39, 0.97, 0.96, 1.0, 0.27),
    (66.0, 61.0, 1.7, 3.8, 0.28, 0.58, 0.82, 1.0, 0.29),
    (72.0, 34.0, 1.3, 5.0, 0.36, 0.76, 0.61, 1.0, 0.24),
    (79.0, 84.0, 1.1, 1.9, 0.42, 0.91, 0.94, 1.0, 0.28),
    (85.0, 18.0, 1.6, 4.6, 0.32, 0.62, 0.86, 1.0, 0.26),
    (91.0, 69.0, 1.3, 2.2, 0.40, 0.83, 0.64, 1.0, 0.24),
    (95.0, 39.0, 1.0, 3.4, 0.34, 0.92, 0.95, 1.0, 0.21),
    (11.0, 47.0, 1.1, 5.4, 0.29, 0.59, 0.83, 1.0, 0.22),
    (25.0, 55.0, 1.4, 1.3, 0.37, 0.93, 0.94, 1.0, 0.20),
    (76.0, 11.0, 1.2, 2.7, 0.34, 0.74, 0.61, 1.0, 0.22),
];

pub struct StartingScreenPlugin;

impl Plugin for StartingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::StartingScreen), setup_starting_screen)
            .add_systems(
                Update,
                animate_starfield.run_if(in_state(GameState::StartingScreen)),
            )
            .add_systems(
                Update,
                handle_menu_buttons
                    .run_if(in_state(GameState::StartingScreen))
                    .run_if(in_state(SettingsState::Closed)),
            );
    }
}

#[derive(Component)]
struct StartingScreenStar {
    phase: f32,
    speed: f32,
    red: f32,
    green: f32,
    blue: f32,
    base_alpha: f32,
}

#[derive(Component, Clone, Copy)]
enum StartingScreenAction {
    NewWorld,
    LoadWorlds,
    Settings,
    ExitGame,
}

fn setup_starting_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        BoxShadowSamples(8),
        DespawnOnExit(GameState::StartingScreen),
    ));

    let logo = asset_server.load("branding/asteria_logo.png");

    commands
        .spawn((
            DespawnOnExit(GameState::StartingScreen),
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(BACKGROUND_COLOR),
            BackgroundGradient(vec![
                RadialGradient {
                    position: UiPosition::CENTER,
                    shape: RadialGradientShape::Circle(vh(72)),
                    stops: vec![
                        ColorStop::auto(Color::srgba(0.30, 0.12, 0.62, 0.28)),
                        ColorStop::auto(Color::srgba(0.18, 0.08, 0.40, 0.14)),
                        ColorStop::auto(Color::srgba(0.08, 0.03, 0.18, 0.0)),
                    ],
                    ..default()
                }
                .into(),
                LinearGradient::to_top_right(vec![
                    ColorStop::auto(Color::srgba(0.04, 0.24, 0.34, 0.14)),
                    ColorStop::auto(Color::srgba(0.02, 0.06, 0.12, 0.0)),
                    ColorStop::auto(Color::srgba(0.20, 0.05, 0.30, 0.10)),
                ])
                .into(),
            ]),
        ))
        .with_children(|parent| {
            for &(left, top, size, phase, speed, red, green, blue, base_alpha) in STAR_FIELD {
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(left),
                        top: percent(top),
                        width: px(size),
                        height: px(size),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(red, green, blue, base_alpha)),
                    StartingScreenStar {
                        phase,
                        speed,
                        red,
                        green,
                        blue,
                        base_alpha,
                    },
                ));
            }

            parent
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|content| {
                    content.spawn((
                        ImageNode::new(logo),
                        Node {
                            width: px(560),
                            margin: UiRect::bottom(px(18)),
                            ..default()
                        },
                    ));

                    content
                        .spawn((
                            Node {
                                width: px(370),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                row_gap: px(12),
                                padding: UiRect::all(px(22)),
                                border_radius: BorderRadius::all(px(10)),
                                ..default()
                            },
                            BackgroundColor(PANEL_COLOR),
                            BackgroundGradient(vec![
                                LinearGradient::to_bottom_right(vec![
                                    ColorStop::auto(Color::srgba(0.43, 0.24, 0.78, 0.12)),
                                    ColorStop::auto(Color::srgba(0.08, 0.07, 0.15, 0.02)),
                                    ColorStop::auto(Color::srgba(0.20, 0.55, 0.68, 0.07)),
                                ])
                                .into(),
                            ]),
                            BoxShadow(vec![
                                ShadowStyle {
                                    color: Color::srgba(0.0, 0.0, 0.0, 0.52),
                                    x_offset: px(0),
                                    y_offset: px(12),
                                    spread_radius: px(2),
                                    blur_radius: px(30),
                                },
                                ShadowStyle {
                                    color: VIOLET_GLOW,
                                    x_offset: px(0),
                                    y_offset: px(0),
                                    spread_radius: px(-4),
                                    blur_radius: px(28),
                                },
                            ]),
                        ))
                        .with_children(|menu| {
                            menu.spawn(menu_button("New World", StartingScreenAction::NewWorld));
                            menu.spawn(menu_button(
                                "Load Worlds",
                                StartingScreenAction::LoadWorlds,
                            ));
                            menu.spawn(menu_button("Settings", StartingScreenAction::Settings));
                            menu.spawn(menu_button("Exit Game", StartingScreenAction::ExitGame));
                        });
                });

            parent.spawn((
                Text::new(format!("v{}", env!("CARGO_PKG_VERSION"))),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                LetterSpacing::Px(0.8),
                TextColor(VERSION_COLOR),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(20),
                    bottom: px(16),
                    ..default()
                },
            ));
        });
}

fn menu_button(label: &'static str, action: StartingScreenAction) -> impl Bundle {
    (
        Button,
        action,
        Node {
            width: px(326),
            height: px(52),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(BUTTON_COLOR),
        button_shadow(Color::srgba(0.0, 0.0, 0.0, 0.28)),
        children![(
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..default()
            },
            LetterSpacing::Px(0.7),
            TextColor(TEXT_COLOR),
        )],
    )
}

fn button_shadow(glow: Color) -> BoxShadow {
    BoxShadow(vec![ShadowStyle {
        color: glow,
        x_offset: px(0),
        y_offset: px(0),
        spread_radius: px(0),
        blur_radius: px(16),
    }])
}

fn animate_starfield(
    time: Res<Time>,
    mut stars: Query<(&StartingScreenStar, &mut BackgroundColor)>,
) {
    let elapsed = time.elapsed_secs();

    for (star, mut background) in &mut stars {
        let shimmer = 0.72 + 0.28 * (elapsed * star.speed + star.phase).sin();
        let alpha = star.base_alpha * shimmer;
        *background = Color::srgba(star.red, star.green, star.blue, alpha).into();
    }
}

fn handle_menu_buttons(
    mut interactions: Query<
        (
            &Interaction,
            &StartingScreenAction,
            &mut BackgroundColor,
            &mut BoxShadow,
        ),
        Changed<Interaction>,
    >,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_settings_state: ResMut<NextState<SettingsState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, action, mut background, mut shadow) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                *background = BUTTON_PRESSED_COLOR.into();
                *shadow = button_shadow(CYAN_GLOW);

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
            Interaction::Hovered => {
                *background = BUTTON_HOVER_COLOR.into();
                *shadow = button_shadow(Color::srgba(0.50, 0.30, 1.0, 0.34));
            }
            Interaction::None => {
                *background = BUTTON_COLOR.into();
                *shadow = button_shadow(Color::srgba(0.0, 0.0, 0.0, 0.28));
            }
        }
    }
}
