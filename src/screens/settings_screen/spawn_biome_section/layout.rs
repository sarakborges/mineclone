use bevy::{
    prelude::*,
    text::{EditableText, FontWeight},
    ui_widgets::ScrollArea,
};

use crate::{
    localization::{Language, UiLocalization},
    ui::{dropdown::{self, PanelAnchor}, text_input, theme, typography},
};

use super::state::SpawnBiomeDropdownKind;

pub(super) const OPTION_GAP: f32 = 4.0;
const SEARCH_HEIGHT: f32 = 40.0;
const VISIBLE_OPTION_COUNT: f32 = 5.0;
const OPTIONS_VIEWPORT_HEIGHT: f32 =
    dropdown::OPTION_HEIGHT * VISIBLE_OPTION_COUNT + OPTION_GAP * (VISIBLE_OPTION_COUNT - 1.0);

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeDropdownButton;

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeDropdownLabel;

#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeDropdownPanel;

/// Owns border/padding; the editable child remains the keyboard focus target.
#[derive(Component)]
pub(in crate::screens::settings_screen) struct SpawnBiomeSearchFrame;

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
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(8),
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
                dropdown::root(percent(100)),
                children![
                    (
                        Button,
                        SpawnBiomeDropdownButton,
                        dropdown::control::<SpawnBiomeDropdownKind>(),
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
                        dropdown::panel_node(percent(100), 8.0, 8.0, 1.0, PanelAnchor::Left),
                        dropdown::panel_surface::<SpawnBiomeDropdownKind>(),
                        GlobalZIndex(610),
                        children![
                            (
                                Button,
                                SpawnBiomeSearchFrame,
                                dropdown::inside::<SpawnBiomeDropdownKind>(),
                                Node {
                                    position_type: PositionType::Relative,
                                    width: percent(100),
                                    height: px(SEARCH_HEIGHT),
                                    padding: UiRect::horizontal(px(text_input::INPUT_PADDING_X)),
                                    border: UiRect::all(px(1)),
                                    align_items: AlignItems::Center,
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                text_input::frame_surface(false),
                                children![
                                    (
                                        Button,
                                        SpawnBiomeSearchBar,
                                        dropdown::inside::<SpawnBiomeDropdownKind>(),
                                        EditableText {
                                            max_characters: Some(128),
                                            ..default()
                                        },
                                        text_input::editor_style(17.0, FontWeight::NORMAL),
                                        Node {
                                            width: percent(100),
                                            min_width: px(0),
                                            height: px(text_input::INPUT_EDITOR_HEIGHT),
                                            overflow: Overflow::clip(),
                                            ..default()
                                        },
                                    ),
                                    (
                                        SpawnBiomeSearchText,
                                        Visibility::Inherited,
                                        Text::new(
                                            localization
                                                .text(language, "newWorld.spawnBiome.search")
                                                .to_owned(),
                                        ),
                                        TextFont {
                                            font: FontSource::SystemUi,
                                            font_size: FontSize::Px(17.0),
                                            ..default()
                                        },
                                        TextColor(theme::TEXT_MUTED),
                                        TextLayout::no_wrap(),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            top: px(text_input::centered_text_top(SEARCH_HEIGHT)),
                                            left: px(text_input::INPUT_PADDING_X + 1.0),
                                            max_width: percent(90),
                                            overflow: Overflow::clip(),
                                            ..default()
                                        },
                                        Pickable::IGNORE,
                                    ),
                                ],
                            ),
                            (
                                SpawnBiomeOptionsFrame,
                                dropdown::inside::<SpawnBiomeDropdownKind>(),
                                Interaction::default(),
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
                                    dropdown::inside::<SpawnBiomeDropdownKind>(),
                                    Interaction::default(),
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
