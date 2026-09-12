use bevy::prelude::*;

use crate::{
    app::settings_state::SettingsState,
    localization::{ActiveLanguage, UiLocalization},
    ui::{
        button::sidebar_menu_button,
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

pub(super) fn section_button(section: SettingsSection, label: impl Into<String>) -> impl Bundle {
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
    mut labels: Query<(&SettingsSectionButtonLabel, &mut Text)>,
) {
    for (panel, mut node) in &mut panels {
        node.display = if panel.0 == selection.selected {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (label, mut text) in &mut labels {
        let next = localization.text(language.get(), label.0.localization_key());
        if text.0 != next {
            text.0 = next.to_owned();
        }
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
