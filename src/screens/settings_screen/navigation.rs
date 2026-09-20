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
    WorldSettings,
    WorldGeneration,
    GameRules,
    Graphics,
    Languages,
    Hud,
}

impl SettingsSection {
    pub(super) const fn localization_key(self) -> &'static str {
        match self {
            Self::WorldSettings => "settings.section.worldSettings",
            Self::WorldGeneration => "newWorld.section.worldGeneration",
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
pub(super) struct SettingsContentScrollArea;

#[derive(Component, Clone, Copy)]
pub(super) struct SettingsPendingSectionScroll(pub(super) SettingsSection);

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

pub(super) fn apply_pending_section_scroll(
    mut commands: Commands,
    mut scroll_areas: Query<
        (
            Entity,
            &SettingsPendingSectionScroll,
            &mut ScrollPosition,
            &ComputedNode,
            &UiGlobalTransform,
        ),
        With<SettingsContentScrollArea>,
    >,
    panels: Query<(&SettingsSectionPanel, &ComputedNode, &UiGlobalTransform)>,
) {
    let Ok((entity, pending, mut scroll, computed, transform)) = scroll_areas.single_mut() else {
        return;
    };
    if scroll_to_section(pending.0, &mut scroll, computed, transform, &panels) {
        commands.entity(entity).remove::<SettingsPendingSectionScroll>();
    }
}

pub(super) fn handle_section_buttons(
    interactions: Query<(&Interaction, &SettingsSectionButton), Changed<Interaction>>,
    mut scroll_areas: Query<
        (&mut ScrollPosition, &ComputedNode, &UiGlobalTransform),
        With<SettingsContentScrollArea>,
    >,
    panels: Query<(&SettingsSectionPanel, &ComputedNode, &UiGlobalTransform)>,
) {
    let Some(section) = interactions.iter().find_map(|(interaction, section)| {
        (*interaction == Interaction::Pressed).then_some(section.0)
    }) else {
        return;
    };
    let Ok((mut scroll, computed, transform)) = scroll_areas.single_mut() else {
        return;
    };

    scroll_to_section(section, &mut scroll, computed, transform, &panels);
}

fn scroll_to_section(
    section: SettingsSection,
    scroll: &mut ScrollPosition,
    scroll_computed: &ComputedNode,
    scroll_transform: &UiGlobalTransform,
    panels: &Query<(&SettingsSectionPanel, &ComputedNode, &UiGlobalTransform)>,
) -> bool {
    let Some((_, panel_computed, panel_transform)) = panels
        .iter()
        .find(|(panel, computed, _)| panel.0 == section && computed.size().y > 0.5)
    else {
        return false;
    };

    let max_offset = (scroll_computed.content_size().y - scroll_computed.size().y).max(0.0)
        * scroll_computed.inverse_scale_factor;
    let target = section_scroll_offset(
        scroll_computed,
        scroll_transform,
        panel_computed,
        panel_transform,
    )
    .clamp(0.0, max_offset);

    if (scroll.0.y - target).abs() > 0.5 {
        scroll.0.y = target;
    }
    true
}

pub(super) fn sync_section_ui(
    mut selection: ResMut<SettingsSectionSelection>,
    scroll_areas: Query<
        (&ScrollPosition, &ComputedNode, &UiGlobalTransform),
        With<SettingsContentScrollArea>,
    >,
    panels: Query<(&SettingsSectionPanel, &ComputedNode, &UiGlobalTransform)>,
    mut buttons: Query<(&SettingsSectionButton, &mut ButtonVariant)>,
) {
    if let Ok((scroll, computed, transform)) = scroll_areas.single() {
        let current = scroll.0.y;
        let mut first = None;
        let mut active = None;

        for (panel, panel_computed, panel_transform) in &panels {
            if panel_computed.size().y <= 0.5 {
                continue;
            }
            let offset =
                section_scroll_offset(computed, transform, panel_computed, panel_transform);

            if first.is_none_or(|(_, first_offset)| offset < first_offset) {
                first = Some((panel.0, offset));
            }
            if offset <= current + 1.0
                && active.is_none_or(|(_, active_offset)| offset > active_offset)
            {
                active = Some((panel.0, offset));
            }
        }

        if let Some((next, _)) = active.or(first)
            && selection.selected != next
        {
            selection.selected = next;
        }
    }

    for (section, mut variant) in &mut buttons {
        *variant = ButtonVariant::from_active(section.0 == selection.selected);
    }
}

fn section_scroll_offset(
    scroll_computed: &ComputedNode,
    scroll_transform: &UiGlobalTransform,
    panel_computed: &ComputedNode,
    panel_transform: &UiGlobalTransform,
) -> f32 {
    let scroll_top = node_top(scroll_computed, scroll_transform);
    let panel_top = node_top(panel_computed, panel_transform);
    ((panel_top - scroll_top) + scroll_computed.scroll_position.y)
        * scroll_computed.inverse_scale_factor
}

fn node_top(computed: &ComputedNode, transform: &UiGlobalTransform) -> f32 {
    let (_, _, translation) = transform.to_scale_angle_translation();
    translation.y - computed.size().y * 0.5
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
