use bevy::prelude::*;

use crate::{
    hud::HudSettings,
    localization::{Language, UiLocalization},
    ui::{theme, typography},
};

const TOGGLE_WIDTH: f32 = 52.0;
const TOGGLE_HEIGHT: f32 = 30.0;
const TOGGLE_THUMB_SIZE: f32 = 20.0;
const TOGGLE_THUMB_INSET: f32 = 4.0;
const TOGGLE_THUMB_ENABLED_LEFT: f32 = TOGGLE_WIDTH - TOGGLE_THUMB_SIZE - TOGGLE_THUMB_INSET;

#[derive(Component)]
pub(super) struct DisplayTooltipsToggle;

#[derive(Component)]
pub(super) struct DisplayTooltipsToggleThumb;

pub(super) fn miscellaneous_section(
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
            (
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(5),
                    ..default()
                },
                children![
                    typography::setting_title(
                        localization
                            .text(language, "settings.displayTooltips")
                            .to_owned(),
                    ),
                    typography::caption(
                        localization
                            .text(language, "settings.displayTooltips.description")
                            .to_owned(),
                    ),
                ],
            ),
            display_tooltips_toggle(display_tooltips),
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
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(toggle_background(enabled, Interaction::None)),
        BorderColor::all(toggle_border(enabled)),
        children![(
            DisplayTooltipsToggleThumb,
            Node {
                position_type: PositionType::Absolute,
                left: px(toggle_thumb_left(enabled)),
                top: px(TOGGLE_THUMB_INSET),
                width: px(TOGGLE_THUMB_SIZE),
                height: px(TOGGLE_THUMB_SIZE),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(theme::TEXT_PRIMARY),
            Pickable::IGNORE,
        )],
    )
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

pub(super) fn sync_display_tooltips_toggle(
    settings: Res<HudSettings>,
    mut toggles: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        With<DisplayTooltipsToggle>,
    >,
    mut thumbs: Query<&mut Node, With<DisplayTooltipsToggleThumb>>,
) {
    let enabled = settings.display_tooltips();

    for (interaction, mut background, mut border) in &mut toggles {
        *background = BackgroundColor(toggle_background(enabled, *interaction));
        *border = BorderColor::all(toggle_border(enabled));
    }

    for mut thumb in &mut thumbs {
        thumb.left = px(toggle_thumb_left(enabled));
    }
}

fn toggle_thumb_left(enabled: bool) -> f32 {
    if enabled {
        TOGGLE_THUMB_ENABLED_LEFT
    } else {
        TOGGLE_THUMB_INSET
    }
}

fn toggle_border(enabled: bool) -> Color {
    if enabled {
        Color::srgba(0.66, 0.52, 0.96, 0.86)
    } else {
        Color::srgba(0.52, 0.50, 0.62, 0.46)
    }
}

fn toggle_background(enabled: bool, interaction: Interaction) -> Color {
    match (enabled, interaction) {
        (true, Interaction::Pressed) => Color::srgba(0.38, 0.24, 0.68, 0.96),
        (true, Interaction::Hovered) => Color::srgba(0.34, 0.22, 0.62, 0.92),
        (true, Interaction::None) => Color::srgba(0.29, 0.19, 0.54, 0.86),
        (false, Interaction::Pressed) => Color::srgba(0.13, 0.11, 0.20, 0.94),
        (false, Interaction::Hovered) => Color::srgba(0.11, 0.09, 0.18, 0.90),
        (false, Interaction::None) => Color::srgba(0.07, 0.06, 0.12, 0.82),
    }
}
