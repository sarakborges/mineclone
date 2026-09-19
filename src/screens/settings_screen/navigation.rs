use bevy::prelude::*;

use crate::{
    app::settings_state::SettingsState,
    ui::{
        button::{button, ButtonVariant, MENU_BUTTON_HEIGHT},
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
    General,
    WorldSettings,
    GameRules,
    Graphics,
    Languages,
    Hud,
}

impl SettingsSection {
    pub(super) const fn localization_key(self) -> &'static str {
        match self {
            Self::General => "newWorld.section.general",
            Self::WorldSettings => "settings.section.worldSettings",
            Self::GameRules => "settings.section.gameRules",
            Self::Graphics => "settings.section.graphics",
            Self::Languages => "settings.section.languages",
            Self::Hud => "settings.section.hud",
        }
    }
}

#[derive(Component, Clone, Copy)]
pub(super) struct SettingsSectionButton(pub(super) SettingsSection);

#[derive(Component, Clone, Copy)]
pub(super) struct SettingsSectionPanel(pub(super) SettingsSection);

#[derive(Component)]
pub(super) struct SettingsBackButton;

pub(super) fn section_button(section: SettingsSection, label: impl Into<String>) -> impl Bundle {
    button(
        label,
        SettingsSectionButton(section),
        percent(100),
        MENU_BUTTON_HEIGHT,
        ButtonVariant::Normal,
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
    mut panels: Query<(&SettingsSectionPanel, &mut Node)>,
    mut buttons: Query<(&SettingsSectionButton, &mut ButtonVariant)>,
) {
    for (section, mut variant) in &mut buttons {
        *variant = ButtonVariant::from_active(section.0 == selection.selected);
    }

    for (panel, mut node) in &mut panels {
        let next_display = if panel.0 == selection.selected {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != next_display {
            node.display = next_display;
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
