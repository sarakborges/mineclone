use bevy::prelude::*;

use crate::{
    app::settings_state::SettingsState,
    localization::{ActiveLanguage, UiLocalization},
    ui::{
        button::sidebar_menu_button,
        theme,
        transition::{ScreenTransition, ScreenTransitionTarget},
    },
};

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SettingsSectionSelection {
    pub(super) selected: SettingsSection,
}

impl Default for SettingsSectionSelection {
    fn default() -> Self {
        Self {
            selected: SettingsSection::WorldSettings,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SettingsSection {
    WorldSettings,
    GameRules,
    Graphics,
    Languages,
    Miscellaneous,
}

impl SettingsSection {
    pub(super) const fn localization_key(self) -> &'static str {
        match self {
            Self::WorldSettings => "settings.section.worldSettings",
            Self::GameRules => "settings.section.gameRules",
            Self::Graphics => "settings.section.graphics",
            Self::Languages => "settings.section.languages",
            Self::Miscellaneous => "settings.section.miscellaneous",
        }
    }
}

#[derive(Component, Clone, Copy)]
pub(super) struct SettingsSectionButton(pub(super) SettingsSection);

#[derive(Component, Clone, Copy)]
pub(super) struct SettingsSectionButtonLabel(pub(super) SettingsSection);

#[derive(Component, Clone, Copy)]
pub(super) struct SettingsSectionPanel(pub(super) SettingsSection);

#[derive(Component)]
pub(super) struct SettingsBackButton;

pub(super) fn section_button(
    section: SettingsSection,
    _active: bool,
    label: impl Into<String>,
) -> impl Bundle {
    sidebar_menu_button(
        label,
        SettingsSectionButton(section),
        SettingsSectionButtonLabel(section),
    )
}

pub(super) fn handle_section_buttons(
    interactions: Query<(&Interaction, &SettingsSectionButton), Changed<Interaction>>,
    mut selection: ResMut<SettingsSectionSelection>,
) {
    for (interaction, section) in &interactions {
        if *interaction == Interaction::Pressed && selection.selected != section.0 {
            selection.selected = section.0;
        }
    }
}

pub(super) fn sync_section_ui(
    selection: Res<SettingsSectionSelection>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut panels: Query<(&SettingsSectionPanel, &mut Node)>,
    mut buttons: Query<(&SettingsSectionButton, &mut BackgroundColor)>,
    mut labels: Query<(&SettingsSectionButtonLabel, &mut Text, &mut TextColor)>,
) {
    for (panel, mut node) in &mut panels {
        node.display = if panel.0 == selection.selected {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (button, mut background) in &mut buttons {
        *background = BackgroundColor(section_button_background(button.0 == selection.selected));
    }

    for (label, mut text, mut color) in &mut labels {
        let next = localization.text(language.get(), label.0.localization_key());
        if text.0 != next {
            text.0 = next.to_owned();
        }
        *color = TextColor(if label.0 == selection.selected {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_MUTED
        });
    }
}

pub(super) fn handle_close_requests(
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<&Interaction, (Changed<Interaction>, With<SettingsBackButton>)>,
    mut transition: ResMut<ScreenTransition>,
) {
    let back_pressed = interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);

    if back_pressed || keys.just_pressed(KeyCode::Escape) {
        transition.request(ScreenTransitionTarget::settings(SettingsState::Closed));
    }
}

fn section_button_background(active: bool) -> Color {
    if active {
        Color::srgba(0.18, 0.10, 0.34, 0.58)
    } else {
        Color::srgba(0.0, 0.0, 0.0, 0.0)
    }
}
