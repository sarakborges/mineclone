use bevy::prelude::*;

use crate::{
    app::{
        game_state::GameState, pause_state::PauseState,
        settings_state::{SettingsScope, SettingsState},
    },
    localization::{ActiveLanguage, UiLocalization},
    ui::{
        button::menu_button,
        theme,
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
    WorldOptions,
    GameOptions,
    LeaveWorld,
    ExitGame,
}

fn pause_group() -> impl Bundle {
    (
        Node {
            width: percent(48),
            min_width: px(0),
            flex_grow: 1.0,
            padding: UiRect::all(px(24)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(14),
            border_radius: BorderRadius::all(px(14)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.09, 0.07, 0.17, 0.92)),
        theme::frosted_surface_gradient(),
    )
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
                padding: UiRect::all(px(20)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(theme::OVERLAY),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: percent(100),
                    max_width: px(1100),
                    padding: UiRect::all(px(30)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(26),
                    border_radius: BorderRadius::all(px(18)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.035, 0.026, 0.075, 0.97)),
                theme::frosted_surface_gradient(),
                BoxShadow(vec![ShadowStyle {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.52),
                    x_offset: px(0),
                    y_offset: px(18),
                    spread_radius: px(0),
                    blur_radius: px(48),
                }]),
            ))
            .with_children(|panel| {
                panel
                    .spawn(Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    })
                    .with_children(|header| {
                        header.spawn(typography::title(
                            localization.text(language, "pause.title").to_owned(),
                        ));
                        header.spawn(typography::caption("ASTERIA"));
                    });

                // Distinct world/game surfaces: no decorative borders on cards.
                panel
                    .spawn(Node {
                        width: percent(100),
                        min_width: px(0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Stretch,
                        column_gap: px(20),
                        ..default()
                    })
                    .with_children(|groups| {
                        groups.spawn(pause_group()).with_children(|world| {
                            world.spawn((
                                Node {
                                    width: px(38),
                                    height: px(4),
                                    margin: UiRect::bottom(px(4)),
                                    border_radius: BorderRadius::all(px(2)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.63, 0.40, 0.96)),
                            ));
                            world.spawn(typography::heading(
                                localization.text(language, "pause.worldGroup").to_owned(),
                            ));
                            world.spawn(menu_button(
                                localization.text(language, "pause.worldOptions").to_owned(),
                                PauseMenuAction::WorldOptions,
                            ));
                            world.spawn(menu_button(
                                localization.text(language, "pause.leaveWorld").to_owned(),
                                PauseMenuAction::LeaveWorld,
                            ));
                        });

                        groups.spawn(pause_group()).with_children(|game| {
                            game.spawn((
                                Node {
                                    width: px(38),
                                    height: px(4),
                                    margin: UiRect::bottom(px(4)),
                                    border_radius: BorderRadius::all(px(2)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.34, 0.80, 0.92)),
                            ));
                            game.spawn(typography::heading(
                                localization.text(language, "pause.gameGroup").to_owned(),
                            ));
                            game.spawn(menu_button(
                                localization.text(language, "pause.resume").to_owned(),
                                PauseMenuAction::Resume,
                            ));
                            game.spawn(menu_button(
                                localization.text(language, "pause.gameOptions").to_owned(),
                                PauseMenuAction::GameOptions,
                            ));
                            game.spawn(menu_button(
                                localization.text(language, "common.exitGame").to_owned(),
                                PauseMenuAction::ExitGame,
                            ));
                        });
                    });

                panel.spawn((PauseSaveFeedback, typography::caption(String::new())));
            });
        });
}

fn handle_pause_menu_buttons(
    interactions: Query<(&Interaction, &PauseMenuAction), Changed<Interaction>>,
    snapshot: WorldSaveContext,
    mut session: ResMut<WorldSession>,
    mut feedback: Query<&mut Text, With<PauseSaveFeedback>>,
    mut scope: ResMut<SettingsScope>,
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
            PauseMenuAction::WorldOptions => {
                *scope = SettingsScope::World;
                transition.request(ScreenTransitionTarget::settings(SettingsState::Open));
            }
            PauseMenuAction::GameOptions => {
                *scope = SettingsScope::Game;
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
