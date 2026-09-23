use bevy::prelude::*;

use crate::{
    hud::{HintKind, HudSettings, TargetBlockPosition},
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{
        dropdown::{self, DropdownState, PanelAnchor},
        selectable, toggle, typography,
    },
};

const DROPDOWN_WIDTH: f32 = 240.0;

pub(super) struct TargetBlockPositionDropdownKind;
pub(super) type TargetBlockPositionDropdownState =
    DropdownState<TargetBlockPositionDropdownKind>;

#[derive(Component)]
pub(super) struct HideHintsToggle;

#[derive(Component)]
pub(super) struct HideHintsToggleThumb;

#[derive(Component, Clone, Copy)]
pub(super) struct HintToggle(HintKind);

#[derive(Component, Clone, Copy)]
pub(super) struct HintToggleThumb(HintKind);

#[derive(Component)]
pub(super) struct TargetBlockPositionDropdownButton;

#[derive(Component)]
pub(super) struct TargetBlockPositionDropdownLabel;

#[derive(Component)]
pub(super) struct TargetBlockPositionDropdownPanel;

#[derive(Component, Clone, Copy)]
pub(super) struct TargetBlockPositionOption(TargetBlockPosition);

#[derive(Component, Clone, Copy)]
pub(super) struct TargetBlockPositionOptionLabel(TargetBlockPosition);

pub(super) fn hud_section(
    settings: &HudSettings,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(18),
            ..default()
        },
        children![
            hide_hints_setting(settings.hide_hints(), localization, language),
            target_block_position_setting(settings.target_block_position(), localization, language),
            hint_setting(HintKind::OpenInventory, settings, localization, language),
            hint_setting(HintKind::CloseInventory, settings, localization, language),
            hint_setting(HintKind::RotateBlock, settings, localization, language),
            hint_setting(HintKind::BreakBlock, settings, localization, language),
            hint_setting(HintKind::BreakOrPlaceBlock, settings, localization, language),
            hint_setting(HintKind::BrushPaint, settings, localization, language),
            hint_setting(HintKind::BrushClear, settings, localization, language),
            hint_setting(HintKind::Chisel, settings, localization, language),
            hint_setting(HintKind::Shears, settings, localization, language),
            hint_setting(HintKind::StructureTool, settings, localization, language),
        ],
    )
}

fn hide_hints_setting(hide_hints: bool, localization: &UiLocalization, language: Language) -> impl Bundle {
    (
        setting_row(),
        children![
            setting_copy(
                localization.text(language, "settings.hideHints"),
                localization.text(language, "settings.hideHints.description"),
            ),
            (
                Button,
                HideHintsToggle,
                toggle::control(hide_hints),
                children![(HideHintsToggleThumb, toggle::thumb(hide_hints))],
            ),
        ],
    )
}

fn hint_setting(
    kind: HintKind,
    settings: &HudSettings,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    let enabled = settings.hint_preference(kind);
    (
        setting_row(),
        children![
            setting_copy(
                localization.text(language, kind.localization_key()),
                localization.text(language, "settings.hint.description"),
            ),
            (
                Button,
                HintToggle(kind),
                toggle::control(enabled),
                children![(HintToggleThumb(kind), toggle::thumb(enabled))],
            ),
        ],
    )
}

fn target_block_position_setting(
    position: TargetBlockPosition,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        setting_row(),
        children![
            setting_copy(
                localization.text(language, "settings.targetBlockPosition"),
                localization.text(language, "settings.targetBlockPosition.description"),
            ),
            target_position_dropdown(position, localization, language),
        ],
    )
}

fn setting_row() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        column_gap: px(18),
        ..default()
    }
}

fn setting_copy(title: impl Into<String>, description: impl Into<String>) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            min_width: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            ..default()
        },
        children![
            typography::setting_title(title),
            typography::caption(description),
        ],
    )
}

fn target_position_dropdown(
    selected: TargetBlockPosition,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        dropdown::root(px(DROPDOWN_WIDTH)),
        children![
            (
                Button,
                TargetBlockPositionDropdownButton,
                dropdown::control::<TargetBlockPositionDropdownKind>(),
                children![
                    (
                        TargetBlockPositionDropdownLabel,
                        typography::hud(target_position_label(selected, localization, language)),
                        Pickable::IGNORE,
                    ),
                    dropdown::indicator(),
                ],
            ),
            (
                TargetBlockPositionDropdownPanel,
                dropdown::panel_node(percent(100), 6.0, 4.0, 1.0, PanelAnchor::Left),
                dropdown::panel_surface::<TargetBlockPositionDropdownKind>(),
                GlobalZIndex(620),
                children![
                    target_position_option(TargetBlockPosition::Center, selected, localization, language),
                    target_position_option(TargetBlockPosition::TopRight, selected, localization, language),
                    target_position_option(TargetBlockPosition::Hidden, selected, localization, language),
                ],
            ),
        ],
    )
}

fn target_position_option(
    position: TargetBlockPosition,
    selected: TargetBlockPosition,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        Button,
        TargetBlockPositionOption(position),
        dropdown::option::<TargetBlockPositionDropdownKind>(position == selected, 1.0),
        children![(
            TargetBlockPositionOptionLabel(position),
            typography::hud(target_position_label(position, localization, language)),
            Pickable::IGNORE,
        )],
    )
}

fn target_position_label(
    position: TargetBlockPosition,
    localization: &UiLocalization,
    language: Language,
) -> String {
    localization
        .text(
            language,
            match position {
                TargetBlockPosition::Center => "settings.targetBlockPosition.center",
                TargetBlockPosition::TopRight => "settings.targetBlockPosition.topRight",
                TargetBlockPosition::Hidden => "settings.targetBlockPosition.hidden",
            },
        )
        .to_owned()
}

pub(super) fn handle_hide_hints_toggle(
    interactions: Query<&Interaction, (Changed<Interaction>, With<HideHintsToggle>)>,
    mut settings: ResMut<HudSettings>,
) {
    if interactions.iter().any(|interaction| *interaction == Interaction::Pressed) {
        let hide_hints = !settings.hide_hints();
        settings.set_hide_hints(hide_hints);
    }
}

pub(super) fn handle_hint_toggles(
    interactions: Query<(&Interaction, &HintToggle), Changed<Interaction>>,
    mut settings: ResMut<HudSettings>,
) {
    for (interaction, hint) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let enabled = !settings.hint_preference(hint.0);
        settings.set_hint_preference(hint.0, enabled);
    }
}

pub(super) fn handle_target_block_position_dropdown_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<TargetBlockPositionDropdownButton>)>,
    mut state: ResMut<TargetBlockPositionDropdownState>,
) {
    if interactions.iter().any(|interaction| *interaction == Interaction::Pressed) {
        state.toggle();
    }
}

pub(super) fn handle_target_block_position_options(
    interactions: Query<(&Interaction, &TargetBlockPositionOption), Changed<Interaction>>,
    mut settings: ResMut<HudSettings>,
    mut state: ResMut<TargetBlockPositionDropdownState>,
) {
    for (interaction, option) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if settings.target_block_position() != option.0 {
            settings.set_target_block_position(option.0);
        }
        state.close();
        break;
    }
}

pub(super) fn close_target_block_position_dropdown_outside(
    mouse: Res<ButtonInput<MouseButton>>,
    inside: Query<&Interaction, With<dropdown::DropdownInside<TargetBlockPositionDropdownKind>>>,
    mut state: ResMut<TargetBlockPositionDropdownState>,
) {
    if dropdown::clicked_outside(state.is_open(), &mouse, &inside) {
        state.close();
    }
}

pub(super) fn sync_hide_hints_toggle(
    settings: Res<HudSettings>,
    mut toggles: Query<(Ref<Interaction>, &mut BackgroundColor, &mut BorderColor), With<HideHintsToggle>>,
    mut thumbs: Query<&mut Node, With<HideHintsToggleThumb>>,
) {
    let settings_changed = settings.is_changed();
    let enabled = settings.hide_hints();
    for (interaction, background, border) in &mut toggles {
        if !settings_changed && !interaction.is_changed() {
            continue;
        }
        toggle::apply_control_colors(enabled, *interaction, background, border);
    }
    if settings_changed {
        let next_left = px(toggle::thumb_left(enabled));
        for mut thumb in &mut thumbs {
            if thumb.left != next_left {
                thumb.left = next_left;
            }
        }
    }
}

pub(super) fn sync_hint_toggles(
    settings: Res<HudSettings>,
    mut toggles: Query<(&HintToggle, Ref<Interaction>, &mut BackgroundColor, &mut BorderColor)>,
    mut thumbs: Query<(&HintToggleThumb, &mut Node)>,
) {
    let settings_changed = settings.is_changed();
    for (hint, interaction, background, border) in &mut toggles {
        if !settings_changed && !interaction.is_changed() {
            continue;
        }
        toggle::apply_control_colors(
            settings.hint_preference(hint.0),
            *interaction,
            background,
            border,
        );
    }
    if settings_changed {
        for (hint, mut node) in &mut thumbs {
            toggle::apply_thumb_position(settings.hint_preference(hint.0), &mut node);
        }
    }
}

pub(super) fn sync_target_block_position_dropdown(
    state: Res<TargetBlockPositionDropdownState>,
    settings: Res<HudSettings>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut panels: Query<&mut Node, With<TargetBlockPositionDropdownPanel>>,
    mut selected_labels: Query<&mut Text, With<TargetBlockPositionDropdownLabel>>,
    mut option_labels: Query<(&TargetBlockPositionOptionLabel, &mut Text), Without<TargetBlockPositionDropdownLabel>>,
) {
    let localization_changed = localization.is_changed() || language.is_changed();
    if state.is_changed() {
        let next_display = if state.is_open() { Display::Flex } else { Display::None };
        for mut panel in &mut panels {
            if panel.display != next_display {
                panel.display = next_display;
            }
        }
    }
    if settings.is_changed() || localization_changed {
        let next = target_position_label(settings.target_block_position(), &localization, language.get());
        for mut text in &mut selected_labels {
            if text.0 != next {
                text.0 = next.clone();
            }
        }
    }
    if localization_changed {
        for (option, mut text) in &mut option_labels {
            let next = target_position_label(option.0, &localization, language.get());
            if text.0 != next {
                text.0 = next;
            }
        }
    }
}

pub(super) fn sync_target_block_position_options(
    settings: Res<HudSettings>,
    changed_interactions: Query<(), (With<TargetBlockPositionOption>, Changed<Interaction>)>,
    mut options: Query<(&TargetBlockPositionOption, &Interaction, &mut BackgroundColor, &mut BorderColor)>,
) {
    if !settings.is_changed() && changed_interactions.is_empty() {
        return;
    }
    let selected = settings.target_block_position();
    for (option, interaction, background, border) in &mut options {
        selectable::apply_colors(
            selectable::colors(*interaction, option.0 == selected),
            background,
            border,
        );
    }
}
