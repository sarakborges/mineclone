use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    localization::{Language, UiLocalization},
    player::{camera::GameplayCamera, game_mode::GameMode},
    ui::{
        button::{button, ButtonVariant, COMPACT_CONTROL_HEIGHT},
        settings as settings_layout, typography,
    },
    world::NewWorldConfig,
};

const GAME_MODE_BUTTON_GAP: f32 = 8.0;

#[derive(Component, Clone, Copy)]
pub(crate) struct GameModeButton(pub(crate) GameMode);

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
        &'static mut ButtonVariant,
    ),
>;

pub(crate) fn world_settings_section(
    game_mode: GameMode,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        settings_layout::setting_column(),
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
    button(
        label,
        GameModeButton(mode),
        Val::Auto,
        COMPACT_CONTROL_HEIGHT,
        ButtonVariant::from_active(mode == current_game_mode),
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

    for (_entity, button, interaction, mut variant) in &mut buttons {
        if !mode_changed && !interaction.is_changed() {
            continue;
        }

        let active = button.0 == current_game_mode;
        *variant = ButtonVariant::from_active(active);
    }

}
