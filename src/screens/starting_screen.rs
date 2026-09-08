use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, settings_state::SettingsState},
    ui::{button::menu_button, theme, typography},
};

const STAR_FIELD: &[(f32, f32, f32, f32, f32, f32, f32, f32, f32)] = &[
    (7.0, 12.0, 1.5, 0.1, 0.38, 0.88, 0.91, 1.0, 0.42),
    (15.0, 73.0, 1.9, 1.7, 0.31, 0.72, 0.61, 1.0, 0.33),
    (22.0, 28.0, 1.3, 2.4, 0.44, 0.67, 0.89, 1.0, 0.37),
    (29.0, 88.0, 1.5, 0.7, 0.35, 0.96, 0.92, 1.0, 0.30),
    (34.0, 16.0, 1.1, 3.2, 0.41, 0.72, 0.64, 1.0, 0.30),
    (41.0, 68.0, 1.6, 4.1, 0.30, 0.90, 0.95, 1.0, 0.34),
    (47.0, 9.0, 1.3, 1.1, 0.43, 0.64, 0.86, 1.0, 0.32),
    (53.0, 82.0, 1.5, 2.9, 0.33, 0.82, 0.66, 1.0, 0.31),
    (59.0, 21.0, 1.1, 0.5, 0.39, 0.97, 0.96, 1.0, 0.34),
    (66.0, 61.0, 1.8, 3.8, 0.28, 0.58, 0.82, 1.0, 0.36),
    (72.0, 34.0, 1.4, 5.0, 0.36, 0.76, 0.61, 1.0, 0.31),
    (79.0, 84.0, 1.2, 1.9, 0.42, 0.91, 0.94, 1.0, 0.35),
    (85.0, 18.0, 1.7, 4.6, 0.32, 0.62, 0.86, 1.0, 0.33),
    (91.0, 69.0, 1.4, 2.2, 0.40, 0.83, 0.64, 1.0, 0.31),
    (95.0, 39.0, 1.1, 3.4, 0.34, 0.92, 0.95, 1.0, 0.28),
    (11.0, 47.0, 1.2, 5.4, 0.29, 0.59, 0.83, 1.0, 0.29),
    (25.0, 55.0, 1.5, 1.3, 0.37, 0.93, 0.94, 1.0, 0.28),
    (76.0, 11.0, 1.3, 2.7, 0.34, 0.74, 0.61, 1.0, 0.30),
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
    base_left: f32,
    base_top: f32,
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
            BackgroundColor(theme::SCREEN_BACKGROUND),
            theme::cosmic_background_gradient(),
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
                        base_left: left,
                        base_top: top,
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
                    row_gap: px(11),
                    ..default()
                })
                .with_children(|content| {
                    content.spawn((
                        ImageNode::new(logo),
                        Node {
                            width: px(560),
                            margin: UiRect::bottom(px(26)),
                            ..default()
                        },
                    ));

                    content.spawn(menu_button("New World", StartingScreenAction::NewWorld));
                    content.spawn(menu_button(
                        "Load Worlds",
                        StartingScreenAction::LoadWorlds,
                    ));
                    content.spawn(menu_button("Settings", StartingScreenAction::Settings));
                    content.spawn(menu_button("Exit Game", StartingScreenAction::ExitGame));
                });

            parent.spawn((
                typography::caption(format!("v{}", env!("CARGO_PKG_VERSION"))),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(20),
                    bottom: px(16),
                    ..default()
                },
            ));
        });
}

fn animate_starfield(
    time: Res<Time>,
    mut stars: Query<(&StartingScreenStar, &mut BackgroundColor, &mut Node)>,
) {
    let elapsed = time.elapsed_secs();

    for (star, mut background, mut node) in &mut stars {
        let shimmer = 0.66 + 0.34 * (elapsed * star.speed * 1.45 + star.phase).sin();
        let alpha = star.base_alpha * shimmer;

        let drift_time = elapsed * (0.055 + star.speed * 0.055);
        let drift_x = (drift_time + star.phase).sin() * 0.95;
        let drift_y = (drift_time * 0.72 + star.phase * 1.37).cos() * 0.62;

        node.left = percent(star.base_left + drift_x);
        node.top = percent(star.base_top + drift_y);
        *background = Color::srgba(star.red, star.green, star.blue, alpha).into();
    }
}

fn handle_menu_buttons(
    interactions: Query<(&Interaction, &StartingScreenAction), Changed<Interaction>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_settings_state: ResMut<NextState<SettingsState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

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
}
