use bevy::prelude::*;

use crate::{
    app::settings_state::SettingsState,
    localization::{ActiveLanguage, UiLocalization},
    ui::{
        theme,
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
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
}

impl SettingsSection {
    pub(super) const fn localization_key(self) -> &'static str {
        match self {
            Self::WorldSettings => "settings.section.worldSettings",
            Self::GameRules => "settings.section.gameRules",
            Self::Graphics => "settings.section.graphics",
            Self::Languages => "settings.section.languages",
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
    active: bool,
    label: impl Into<String>,
) -> impl Bundle {
    (
        Button,
        SettingsSectionButton(section),
        Node {
            width: percent(100),
            min_height: px(48),
            padding: UiRect::axes(px(16), px(10)),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(section_button_background(active, Interaction::None)),
        children![(
            typography::button_label(label),
            SettingsSectionButtonLabel(section),
        )],
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
    mut buttons: Query<(&SettingsSectionButton, &Interaction, &mut BackgroundColor)>,
    mut labels: Query<(&SettingsSectionButtonLabel, &mut Text, &mut TextColor)>,
) {
    for (panel, mut node) in &mut panels {
        node.display = if panel.0 == selection.selected {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (button, interaction, mut background) in &mut buttons {
        *background = BackgroundColor(section_button_background(
            button.0 == selection.selected,
            *interaction,
        ));
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

fn section_button_background(active: bool, interaction: Interaction) -> Color {
    if active {
        return Color::srgba(0.31, 0.20, 0.56, 0.86);
    }

    match interaction {
        Interaction::Pressed => Color::srgba(0.26, 0.18, 0.48, 0.84),
        Interaction::Hovered => Color::srgba(0.20, 0.14, 0.38, 0.78),
        Interaction::None => Color::srgba(0.08, 0.06, 0.16, 0.34),
    }
}
