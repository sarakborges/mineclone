use bevy::prelude::*;

use crate::{
    app::keybinds::{KeybindAction, KeybindSetError, Keybinds, KeyboardKey},
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{
        button::{button, ButtonVariant, COMPACT_CONTROL_HEIGHT},
        typography,
    },
};

const KEY_BUTTON_WIDTH: f32 = 210.0;

#[derive(Resource, Default)]
pub(super) struct KeybindCaptureState {
    action: Option<KeybindAction>,
    error: String,
}

impl KeybindCaptureState {
    pub(super) fn reset(&mut self) {
        self.action = None;
        self.error.clear();
    }
}

#[derive(Component, Clone, Copy)]
pub(super) struct KeybindButton(KeybindAction);

#[derive(Component)]
pub(super) struct KeybindError;

pub(super) fn keybinds_section(
    keybinds: &Keybinds,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(12),
            ..default()
        },
        children![
            keybind_row(KeybindAction::Jump, keybinds, localization, language),
            keybind_row(KeybindAction::Descend, keybinds, localization, language),
            keybind_row(KeybindAction::Inventory, keybinds, localization, language),
            keybind_row(KeybindAction::CharacterInfo, keybinds, localization, language),
            keybind_row(KeybindAction::Chat, keybinds, localization, language),
            keybind_row(KeybindAction::ToolAction, keybinds, localization, language),
            keybind_row(KeybindAction::ChangePerspective, keybinds, localization, language),
            (
                KeybindError,
                typography::caption(String::new()),
                Node {
                    display: Display::None,
                    ..default()
                },
            ),
        ],
    )
}

fn keybind_row(
    action: KeybindAction,
    keybinds: &Keybinds,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(18),
            ..default()
        },
        children![
            typography::setting_title(
                localization.text(language, action.localization_key()).to_owned()
            ),
            button(
                keybinds.label(action),
                KeybindButton(action),
                px(KEY_BUTTON_WIDTH),
                COMPACT_CONTROL_HEIGHT,
                ButtonVariant::Normal,
            ),
        ],
    )
}

pub(super) fn handle_keybind_buttons(
    interactions: Query<(&Interaction, &KeybindButton), Changed<Interaction>>,
    mut state: ResMut<KeybindCaptureState>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            state.action = Some(button.0);
            state.error.clear();
            break;
        }
    }
}

pub(super) fn handle_keybind_capture(
    keys: Res<ButtonInput<KeyCode>>,
    mut keybinds: ResMut<Keybinds>,
    mut state: ResMut<KeybindCaptureState>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    let Some(action) = state.action else {
        return;
    };

    if keys.just_pressed(KeyCode::Escape) {
        state.reset();
        return;
    }

    let Some(key_code) = keys.get_just_pressed().next().copied() else {
        return;
    };
    let Some(key) = KeyboardKey::from_key_code(key_code) else {
        state.error = localization
            .text(language.get(), "settings.keybind.unsupported")
            .to_owned();
        return;
    };

    match keybinds.set(action, key) {
        Ok(()) => state.reset(),
        Err(KeybindSetError::Reserved) => {
            state.error = localization
                .text(language.get(), "settings.keybind.reserved")
                .replace("{key}", key.label());
        }
        Err(KeybindSetError::Conflict(conflict)) => {
            state.error = localization
                .text(language.get(), "settings.keybind.conflict")
                .replace("{key}", key.label())
                .replace(
                    "{action}",
                    localization.text(language.get(), conflict.localization_key()),
                );
        }
    }
}

pub(super) fn sync_keybinds_section(
    keybinds: Res<Keybinds>,
    state: Res<KeybindCaptureState>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    buttons: Query<(&KeybindButton, &Children)>,
    mut labels: Query<&mut Text, Without<KeybindError>>,
    mut errors: Query<(&mut Text, &mut Node), With<KeybindError>>,
) {
    if !keybinds.is_changed()
        && !state.is_changed()
        && !localization.is_changed()
        && !language.is_changed()
    {
        return;
    }

    for (button, children) in &buttons {
        let Some(&label_entity) = children.first() else {
            continue;
        };
        let Ok(mut label) = labels.get_mut(label_entity) else {
            continue;
        };

        let next = if state.action == Some(button.0) {
            localization
                .text(language.get(), "settings.keybind.pressKey")
                .to_owned()
        } else {
            keybinds.label(button.0).to_owned()
        };
        if label.0 != next {
            label.0 = next;
        }
    }

    for (mut text, mut node) in &mut errors {
        if text.0 != state.error {
            text.0.clone_from(&state.error);
        }
        let display = if state.error.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
    }
}
