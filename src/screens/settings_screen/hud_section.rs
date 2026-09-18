use bevy::prelude::*;

use crate::{
    hud::{HudSettings, TargetBlockPosition},
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{dropdown, surface, theme, typography},
};

use super::navigation::{SettingsSection, SettingsSectionSelection};

const TOGGLE_WIDTH: f32 = 52.0;
const TOGGLE_HEIGHT: f32 = 30.0;
const TOGGLE_THUMB_SIZE: f32 = 20.0;
const TOGGLE_THUMB_INSET: f32 = 3.0;
const TOGGLE_THUMB_RIGHT: f32 = 0.0;
const DROPDOWN_WIDTH: f32 = 240.0;
const DROPDOWN_HEIGHT: f32 = 44.0;
const DROPDOWN_GAP: f32 = 6.0;
const OPTION_HEIGHT: f32 = 40.0;

#[derive(Resource, Default)]
pub(super) struct TargetBlockPositionDropdownState {
    open: bool,
}

#[derive(Component)]
pub(super) struct DisplayTooltipsToggle;

#[derive(Component)]
pub(super) struct DisplayTooltipsToggleThumb;

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
            display_tooltips_setting(settings.display_tooltips(), localization, language),
            target_block_position_setting(
                settings.target_block_position(),
                localization,
                language,
            ),
        ],
    )
}

fn display_tooltips_setting(
    display_tooltips: bool,
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
            setting_copy(
                localization.text(language, "settings.displayTooltips"),
                localization.text(language, "settings.displayTooltips.description"),
            ),
            display_tooltips_toggle(display_tooltips),
        ],
    )
}

fn target_block_position_setting(
    position: TargetBlockPosition,
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
            setting_copy(
                localization.text(language, "settings.targetBlockPosition"),
                localization.text(language, "settings.targetBlockPosition.description"),
            ),
            target_position_dropdown(position, localization, language),
        ],
    )
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

fn display_tooltips_toggle(enabled: bool) -> impl Bundle {
    (
        Button,
        DisplayTooltipsToggle,
        Node {
            width: px(TOGGLE_WIDTH),
            height: px(TOGGLE_HEIGHT),
            flex_shrink: 0.0,
            position_type: PositionType::Relative,
            border: UiRect::all(px(2)),
            ..default()
        },
        BackgroundColor(toggle_background(enabled, Interaction::None)),
        BorderColor::all(toggle_border(enabled)),
        children![(
            DisplayTooltipsToggleThumb,
            Node {
                position_type: PositionType::Absolute,
                right: px(TOGGLE_THUMB_RIGHT),
                top: px(TOGGLE_THUMB_INSET),
                width: px(TOGGLE_THUMB_SIZE),
                height: px(TOGGLE_THUMB_SIZE),
                ..default()
            },
            BackgroundColor(theme::TEXT_PRIMARY),
            Pickable::IGNORE,
        )],
    )
}

fn target_position_dropdown(
    selected: TargetBlockPosition,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    let (background, border) = surface::hud_control_static(false);

    (
        Node {
            position_type: PositionType::Relative,
            width: px(DROPDOWN_WIDTH),
            height: px(DROPDOWN_HEIGHT),
            flex_shrink: 0.0,
            ..default()
        },
        children![
            (
                Button,
                TargetBlockPositionDropdownButton,
                Node {
                    width: percent(100),
                    height: percent(100),
                    padding: UiRect::horizontal(px(12)),
                    border: UiRect::all(px(2)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                BackgroundColor(background),
                BorderColor::all(border),
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
                Node {
                    display: Display::None,
                    position_type: PositionType::Absolute,
                    top: px(DROPDOWN_HEIGHT + DROPDOWN_GAP),
                    left: px(0),
                    width: percent(100),
                    padding: UiRect::all(px(6)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(4),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BackgroundColor(theme::HUD_SURFACE),
                BorderColor::all(surface::HUD_BORDER_COLOR),
                GlobalZIndex(620),
                children![
                    target_position_option(
                        TargetBlockPosition::Center,
                        selected,
                        localization,
                        language,
                    ),
                    target_position_option(
                        TargetBlockPosition::TopRight,
                        selected,
                        localization,
                        language,
                    ),
                    target_position_option(
                        TargetBlockPosition::Hidden,
                        selected,
                        localization,
                        language,
                    ),
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
    let (background, border) = surface::hud_control_static(position == selected);

    (
        Button,
        TargetBlockPositionOption(position),
        Node {
            width: percent(100),
            height: px(OPTION_HEIGHT),
            min_height: px(OPTION_HEIGHT),
            padding: UiRect::horizontal(px(10)),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(background),
        BorderColor::all(border),
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

pub(super) fn handle_display_tooltips_toggle(
    interactions: Query<&Interaction, (Changed<Interaction>, With<DisplayTooltipsToggle>)>,
    mut settings: ResMut<HudSettings>,
) {
    if interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        let enabled = settings.display_tooltips();
        settings.set_display_tooltips(!enabled);
    }
}

pub(super) fn handle_target_block_position_dropdown_button(
    interactions: Query<
        &Interaction,
        (Changed<Interaction>, With<TargetBlockPositionDropdownButton>),
    >,
    mut state: ResMut<TargetBlockPositionDropdownState>,
) {
    if interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        state.open = !state.open;
    }
}

pub(super) fn handle_target_block_position_options(
    interactions: Query<
        (&Interaction, &TargetBlockPositionOption),
        Changed<Interaction>,
    >,
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
        state.open = false;
        break;
    }
}

pub(super) fn close_target_block_position_dropdown_outside_hud(
    selection: Res<SettingsSectionSelection>,
    mouse: Res<ButtonInput<MouseButton>>,
    button: Query<&Interaction, With<TargetBlockPositionDropdownButton>>,
    options: Query<&Interaction, With<TargetBlockPositionOption>>,
    mut state: ResMut<TargetBlockPositionDropdownState>,
) {
    if selection.is_changed() && selection.selected != SettingsSection::Hud && state.open {
        state.open = false;
        return;
    }
    if !state.open || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let clicked_inside = button.iter().any(|i| *i == Interaction::Pressed)
        || options.iter().any(|i| *i == Interaction::Pressed);
    if !clicked_inside {
        state.open = false;
    }
}

pub(super) fn sync_display_tooltips_toggle(
    settings: Res<HudSettings>,
    mut toggles: Query<
        (Ref<Interaction>, &mut BackgroundColor, &mut BorderColor),
        With<DisplayTooltipsToggle>,
    >,
    mut thumbs: Query<&mut Node, With<DisplayTooltipsToggleThumb>>,
) {
    let settings_changed = settings.is_changed();
    let enabled = settings.display_tooltips();

    for (interaction, background, border) in &mut toggles {
        if !settings_changed && !interaction.is_changed() {
            continue;
        }

        surface::apply_control_colors(
            (toggle_background(enabled, *interaction), toggle_border(enabled)),
            background,
            border,
        );
    }

    if !settings_changed {
        return;
    }

    let next_right = px(TOGGLE_THUMB_RIGHT);
    for mut thumb in &mut thumbs {
        if thumb.right != next_right {
            thumb.right = next_right;
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
    mut option_labels: Query<
        (&TargetBlockPositionOptionLabel, &mut Text),
        Without<TargetBlockPositionDropdownLabel>,
    >,
) {
    let localization_changed = localization.is_changed() || language.is_changed();
    if state.is_changed() {
        let next_display = if state.open {
            Display::Flex
        } else {
            Display::None
        };
        for mut panel in &mut panels {
            if panel.display != next_display {
                panel.display = next_display;
            }
        }
    }

    if settings.is_changed() || localization_changed {
        let next = target_position_label(
            settings.target_block_position(),
            &localization,
            language.get(),
        );
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
    mut options: Query<(
        &TargetBlockPositionOption,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    if !settings.is_changed() && changed_interactions.is_empty() {
        return;
    }

    for (option, interaction, background, border) in &mut options {
        let selected = option.0 == settings.target_block_position();
        surface::apply_control_colors(
            surface::hud_control_colors(*interaction, selected),
            background,
            border,
        );
    }
}

const fn toggle_thumb_left(_enabled: bool) -> f32 {
    TOGGLE_WIDTH - TOGGLE_THUMB_SIZE - TOGGLE_THUMB_RIGHT
}

fn toggle_border(enabled: bool) -> Color {
    if enabled {
        theme::BORDER_FOCUS
    } else {
        theme::BORDER
    }
}

fn toggle_background(enabled: bool, interaction: Interaction) -> Color {
    match (enabled, interaction) {
        (true, Interaction::Pressed) => theme::PURPLE,
        (true, Interaction::Hovered) => theme::PURPLE_HOVER,
        (true, Interaction::None) => theme::PURPLE_SOFT,
        (false, Interaction::Pressed) => theme::SURFACE_INSET,
        (false, Interaction::Hovered) => theme::SURFACE_ELEVATED,
        (false, Interaction::None) => theme::HUD_SURFACE,
    }
}
