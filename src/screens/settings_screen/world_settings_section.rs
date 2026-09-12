use bevy::{prelude::*, ui::InteractionDisabled};

use crate::{
    app::game_state::GameState,
    localization::{Language, UiLocalization},
    player::{camera::GameplayCamera, game_mode::GameMode},
    ui::{theme, typography},
    world::NewWorldConfig,
};

const GAME_MODE_BUTTON_HEIGHT: f32 = 44.0;
const GAME_MODE_BUTTON_GAP: f32 = 12.0;

#[derive(Component, Clone, Copy)]
pub(crate) struct GameModeButton(pub(crate) GameMode);

#[derive(Component, Clone, Copy)]
pub(crate) struct GameModeButtonLabel(GameMode);

pub(crate) fn world_settings_section(
    game_mode: GameMode,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(10),
            ..default()
        },
        children![
            typography::setting_title(localization.text(language, "settings.gameMode").to_owned()),
            typography::caption(
                localization
                    .text(language, "settings.gameMode.description")
                    .to_owned(),
            ),
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(GAME_MODE_BUTTON_GAP),
                    ..default()
                },
                children![
                    game_mode_button(
                        localization
                            .text(language, "settings.gameMode.survival")
                            .to_owned(),
                        GameMode::Survival,
                        game_mode,
                    ),
                    game_mode_button(
                        localization
                            .text(language, "settings.gameMode.creative")
                            .to_owned(),
                        GameMode::Creative,
                        game_mode,
                    ),
                ],
            ),
        ],
    )
}

fn game_mode_button(
    label: impl Into<String>,
    mode: GameMode,
    current_game_mode: GameMode,
) -> impl Bundle {
    let active = mode == current_game_mode;

    (
        Button,
        GameModeButton(mode),
        Node {
            flex_grow: 1.0,
            height: px(GAME_MODE_BUTTON_HEIGHT),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(game_mode_button_background(active, Interaction::None)),
        children![(typography::button_label(label), GameModeButtonLabel(mode))],
    )
}

pub(crate) fn handle_game_mode_buttons(
    interactions: Query<
        (&Interaction, &GameModeButton),
        (Changed<Interaction>, Without<InteractionDisabled>),
    >,
    game_state: Res<State<GameState>>,
    mut new_world: ResMut<NewWorldConfig>,
    mut player: Query<&mut GameMode, With<GameplayCamera>>,
) {
    for (interaction, button) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if *game_state.get() == GameState::NewWorld {
            if new_world.game_mode() != button.0 {
                new_world.set_game_mode(button.0);
            }
            continue;
        }

        let Ok(mut current_game_mode) = player.single_mut() else {
            continue;
        };
        if *current_game_mode != button.0 {
            *current_game_mode = button.0;
        }
    }
}

pub(crate) fn sync_game_mode_buttons(
    mut commands: Commands,
    game_state: Res<State<GameState>>,
    new_world: Res<NewWorldConfig>,
    player: Query<&GameMode, With<GameplayCamera>>,
    mut buttons: Query<(
        Entity,
        &GameModeButton,
        &Interaction,
        Has<InteractionDisabled>,
        &mut BackgroundColor,
    )>,
    mut labels: Query<(&GameModeButtonLabel, &mut TextColor)>,
) {
    let current_game_mode = if *game_state.get() == GameState::NewWorld {
        new_world.game_mode()
    } else {
        let Ok(current_game_mode) = player.single() else {
            return;
        };
        *current_game_mode
    };

    for (entity, button, interaction, disabled, mut background) in &mut buttons {
        let active = button.0 == current_game_mode;

        if active && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        } else if !active && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        }

        *background = BackgroundColor(game_mode_button_background(active, *interaction));
    }

    for (label, mut color) in &mut labels {
        *color = TextColor(if label.0 == current_game_mode {
            theme::TEXT_SUBTLE
        } else {
            theme::TEXT_PRIMARY
        });
    }
}

fn game_mode_button_background(active: bool, interaction: Interaction) -> Color {
    if active {
        return Color::srgba(0.08, 0.07, 0.12, 0.62);
    }

    match interaction {
        Interaction::Pressed => Color::srgba(0.34, 0.22, 0.62, 0.92),
        Interaction::Hovered => Color::srgba(0.29, 0.19, 0.54, 0.82),
        Interaction::None => Color::srgba(0.20, 0.14, 0.38, 0.72),
    }
}
