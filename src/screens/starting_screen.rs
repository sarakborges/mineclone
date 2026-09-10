use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, settings_state::SettingsState},
    ui::{
        button::menu_button,
        cosmic_background::{self, STAR_FIELD},
        theme,
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
    },
    world::{
        biome::CurrentBiome, dimension::CurrentDimension, InMemoryWorldSave, WorldLoadMode,
        WorldSeed,
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
            for &spec in STAR_FIELD {
                parent.spawn(cosmic_background::star(spec));
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
                    content.spawn(menu_button("Load Worlds", StartingScreenAction::LoadWorlds));
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

fn handle_menu_buttons(
    mut commands: Commands,
    interactions: Query<(&Interaction, &StartingScreenAction), Changed<Interaction>>,
    save: Res<InMemoryWorldSave>,
    mut transition: ResMut<ScreenTransition>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            StartingScreenAction::NewWorld => {
                commands.insert_resource(CurrentDimension::default());
                commands.insert_resource(CurrentBiome::default());
                commands.insert_resource(WorldSeed::fresh());
                commands.insert_resource(WorldLoadMode::New);
                transition.request(ScreenTransitionTarget::game(GameState::Loading));
            }
            StartingScreenAction::LoadWorlds => {
                let (Some(seed), Some(dimension_id)) = (save.seed(), save.dimension_id()) else {
                    continue;
                };

                commands.insert_resource(WorldSeed(seed.0));
                commands.insert_resource(CurrentDimension {
                    id: dimension_id.to_owned(),
                });
                commands.insert_resource(WorldLoadMode::Load);
                transition.request(ScreenTransitionTarget::game(GameState::Loading));
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
