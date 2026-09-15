use bevy::{prelude::*, ui_widgets::ScrollArea};

use crate::{
    localization::{Language, UiLocalization},
    ui::{dropdown, surface, theme, typography},
};

pub(super) const OPTION_HEIGHT: f32 = 40.0;
pub(super) const OPTION_GAP: f32 = 4.0;
const CONTROL_HEIGHT: f32 = 44.0;
const PANEL_GAP: f32 = 6.0;
const SEARCH_HEIGHT: f32 = 40.0;
const VISIBLE_OPTION_COUNT: f32 = 5.0;
const OPTIONS_VIEWPORT_HEIGHT: f32 =
    OPTION_HEIGHT * VISIBLE_OPTION_COUNT + OPTION_GAP * (VISIBLE_OPTION_COUNT - 1.0);

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeDropdownButton;

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeDropdownLabel;

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeDropdownPanel;

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeSearchBar;

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeSearchText;

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeOptionsFrame;

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeOptionsList;

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeOption {
    pub(super) biome_id: Option<String>,
}

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeOptionLabel {
    pub(super) biome_id: Option<String>,
}

pub(in crate::screens::settings_screen) fn spawn_biome_setting(
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    let (control_background, control_border) = surface::hud_control_static(false);

    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(10),
            ..default()
        },
        children![
            typography::setting_title(
                localization
                    .text(language, "newWorld.spawnBiome")
                    .to_owned(),
            ),
            typography::caption(
                localization
                    .text(language, "newWorld.spawnBiome.description")
                    .to_owned(),
            ),
            (
                Node {
                    position_type: PositionType::Relative,
                    width: percent(100),
                    height: px(CONTROL_HEIGHT),
                    ..default()
                },
                children![
                    (
                        Button,
                        SpawnBiomeDropdownButton,
                        Node {
                            width: percent(100),
                            height: percent(100),
                            padding: UiRect::horizontal(px(12)),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(6)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            ..default()
                        },
                        BackgroundColor(control_background),
                        BorderColor::all(control_border),
                        children![
                            (
                                SpawnBiomeDropdownLabel,
                                typography::hud(
                                    localization
                                        .text(language, "newWorld.spawnBiome.random")
                                        .to_owned(),
                                ),
                                Pickable::IGNORE,
                            ),
                            dropdown::indicator(),
                        ],
                    ),
                    (
                        SpawnBiomeDropdownPanel,
                        Node {
                            display: Display::None,
                            position_type: PositionType::Absolute,
                            top: px(CONTROL_HEIGHT + PANEL_GAP),
                            left: px(0),
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            row_gap: px(8),
                            padding: UiRect::all(px(8)),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(6)),
                            ..default()
                        },
                        BackgroundColor(theme::HUD_SURFACE),
                        BorderColor::all(surface::HUD_BORDER_COLOR),
                        GlobalZIndex(610),
                        children![
                            (
                                Button,
                                SpawnBiomeSearchBar,
                                Node {
                                    width: percent(100),
                                    height: px(SEARCH_HEIGHT),
                                    padding: UiRect::horizontal(px(11)),
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(px(5)),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(theme::SLIDER_TRACK),
                                BorderColor::all(surface::HUD_SELECTED_BORDER_COLOR),
                                children![(
                                    SpawnBiomeSearchText,
                                    typography::caption(
                                        localization
                                            .text(language, "newWorld.spawnBiome.search")
                                            .to_owned(),
                                    ),
                                    Pickable::IGNORE,
                                )],
                            ),
                            (
                                SpawnBiomeOptionsFrame,
                                Node {
                                    display: Display::Grid,
                                    width: percent(100),
                                    height: px(OPTIONS_VIEWPORT_HEIGHT),
                                    min_height: px(OPTIONS_VIEWPORT_HEIGHT),
                                    grid_template_columns: vec![
                                        RepeatedGridTrack::flex(1, 1.0),
                                        RepeatedGridTrack::auto(1),
                                    ],
                                    ..default()
                                },
                                children![(
                                    SpawnBiomeOptionsList,
                                    ScrollPosition(Vec2::ZERO),
                                    ScrollArea,
                                    Node {
                                        width: percent(100),
                                        height: percent(100),
                                        min_height: px(0),
                                        padding: UiRect::right(px(8)),
                                        flex_direction: FlexDirection::Column,
                                        align_items: AlignItems::Stretch,
                                        row_gap: px(OPTION_GAP),
                                        overflow: Overflow::scroll_y(),
                                        ..default()
                                    },
                                )],
                            ),
                        ],
                    ),
                ],
            ),
        ],
    )
}
