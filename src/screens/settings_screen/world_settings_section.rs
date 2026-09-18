use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    localization::{Language, UiLocalization},
    player::{camera::GameplayCamera, game_mode::GameMode},
    ui::{
        selectable::{
            selectable_button_background, selectable_label_color, sync_selectable_button,
        },
        typography,
    },
    world::NewWorldConfig,
};

const GAME_MODE_BUTTON_HEIGHT: f32 = 44.0;
const GAME_MODE_BUTTON_GAP: f32 = 8.0;

#[derive(Component, Clone, Copy)]
pub(crate) struct GameModeButton(pub(crate) GameMode);

#[derive(Component, Clone, Copy)]
struct GameModeButtonLabel(GameMode);

pub(crate) type GameModeButtonInteractions<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static GameModeButton),
    Changed<Interaction>,
>;

type GameModeButtonSyncQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GameModeButton,
        Ref<'static, Interaction>,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
    ),
>;

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
            row_gap: px(8),
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
            border: UiRect::all(px(2)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(selectable_button_background(active, Interaction::None)),
        BorderColor::all(crate::ui::selectable::selectable_button_border(
            active,
            Interaction::None,
        )),
        children![(typography::button_label(label), GameModeButtonLabel(mode))],
    )
}

pub(crate) fn handle_game_mode_buttons(
    interactions: GameModeButtonInteractions,
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
    game_state: Res<State<GameState>>,
    new_world: Res<NewWorldConfig>,
    player: Query<Ref<GameMode>, With<GameplayCamera>>,
    mut buttons: GameModeButtonSyncQuery,
    mut labels: Query<(&GameModeButtonLabel, &mut TextColor)>,
) {
    let (current_game_mode, mode_changed) = if *game_state.get() == GameState::NewWorld {
        (
            new_world.game_mode(),
            game_state.is_changed() || new_world.is_changed(),
        )
    } else {
        let Ok(current_game_mode) = player.single() else {
            return;
        };
        (
            *current_game_mode,
            game_state.is_changed() || current_game_mode.is_changed(),
        )
    };

    for (entity, button, interaction, background, border) in &mut buttons {
        if !mode_changed && !interaction.is_changed() {
            continue;
        }

        sync_selectable_button(
            entity,
            button.0 == current_game_mode,
            false,
            *interaction,
            background,
            border,
        );
    }

    if !mode_changed {
        return;
    }

    for (label, mut color) in &mut labels {
        let next_color = TextColor(selectable_label_color(label.0 == current_game_mode));
        if *color != next_color {
            *color = next_color;
        }
    }
}
