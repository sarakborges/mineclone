use bevy::{prelude::*, ui::InteractionDisabled};

use crate::{
    player::{camera::GameplayCamera, game_mode::GameMode},
    ui::{surface, theme, typography},
};

const GAME_MODE_BUTTON_HEIGHT: f32 = 44.0;
const GAME_MODE_BUTTON_GAP: f32 = 12.0;

#[derive(Component, Clone, Copy)]
pub(super) struct GameModeButton(pub(super) GameMode);

#[derive(Component, Clone, Copy)]
struct GameModeButtonLabel(GameMode);

pub(super) fn world_settings_section(game_mode: GameMode) -> impl Bundle {
    (
        surface::settings_section(),
        children![
            typography::heading("World Settings"),
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    ..default()
                },
                children![
                    typography::muted("Game Mode"),
                    (
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            column_gap: px(GAME_MODE_BUTTON_GAP),
                            ..default()
                        },
                        children![
                            game_mode_button("Survival", GameMode::Survival, game_mode),
                            game_mode_button("Creative", GameMode::Creative, game_mode),
                        ],
                    ),
                ],
            ),
        ],
    )
}

fn game_mode_button(
    label: &'static str,
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

pub(super) fn handle_game_mode_buttons(
    interactions: Query<
        (&Interaction, &GameModeButton),
        (Changed<Interaction>, Without<InteractionDisabled>),
    >,
    mut player: Query<&mut GameMode, With<GameplayCamera>>,
) {
    let Ok(mut current_game_mode) = player.single_mut() else {
        return;
    };

    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed && *current_game_mode != button.0 {
            *current_game_mode = button.0;
        }
    }
}

pub(super) fn sync_game_mode_buttons(
    mut commands: Commands,
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
    let Ok(current_game_mode) = player.single() else {
        return;
    };

    for (entity, button, interaction, disabled, mut background) in &mut buttons {
        let active = button.0 == *current_game_mode;

        if active && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        } else if !active && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        }

        *background = BackgroundColor(game_mode_button_background(active, *interaction));
    }

    for (label, mut color) in &mut labels {
        *color = TextColor(if label.0 == *current_game_mode {
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
