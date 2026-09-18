use bevy::prelude::*;

use crate::{
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{
        dropdown::{self, DropdownState, PanelAnchor},
        selectable, typography,
    },
};

const LANGUAGE_DROPDOWN_WIDTH: f32 = 240.0;

pub(super) struct LanguageDropdownKind;
pub(super) type LanguageDropdownState = DropdownState<LanguageDropdownKind>;

#[derive(Component)]
pub(super) struct LanguageDropdownButton;

#[derive(Component)]
pub(super) struct LanguageDropdownLabel;

#[derive(Component)]
pub(super) struct LanguageDropdownPanel;

#[derive(Component, Clone, Copy)]
pub(super) struct LanguageOption(pub(super) Language);

#[derive(Component, Clone, Copy)]
pub(super) struct LanguageOptionLabel(Language);

pub(super) fn languages_section(
    localization: &UiLocalization,
    active_language: Language,
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
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(5),
                    ..default()
                },
                children![
                    typography::setting_title(
                        localization
                            .text(active_language, "settings.language")
                            .to_owned()
                    ),
                    typography::caption(
                        localization
                            .text(active_language, "settings.language.description")
                            .to_owned()
                    ),
                ],
            ),
            language_dropdown(active_language, localization),
        ],
    )
}

fn language_dropdown(selected: Language, localization: &UiLocalization) -> impl Bundle {
    let panel_node = dropdown::panel_node(
        px(LANGUAGE_DROPDOWN_WIDTH),
        6.0,
        4.0,
        2.0,
        PanelAnchor::Right,
    );

    (
        dropdown::root(px(LANGUAGE_DROPDOWN_WIDTH)),
        children![
            (
                Button,
                LanguageDropdownButton,
                dropdown::control::<LanguageDropdownKind>(),
                children![
                    (
                        LanguageDropdownLabel,
                        typography::hud(language_label(selected, localization)),
                        Pickable::IGNORE,
                    ),
                    dropdown::indicator(),
                ],
            ),
            (
                LanguageDropdownPanel,
                panel_node,
                dropdown::panel_surface::<LanguageDropdownKind>(),
                GlobalZIndex(620),
                children![
                    language_option(Language::English, selected, localization),
                    language_option(Language::PortugueseBrazil, selected, localization),
                    language_option(Language::Spanish, selected, localization),
                ],
            ),
        ],
    )
}

fn language_option(
    language: Language,
    selected: Language,
    localization: &UiLocalization,
) -> impl Bundle {
    (
        Button,
        LanguageOption(language),
        dropdown::option::<LanguageDropdownKind>(language == selected, 2.0),
        children![(
            LanguageOptionLabel(language),
            typography::hud(language_label(language, localization)),
            Pickable::IGNORE,
        )],
    )
}

fn language_label(language: Language, localization: &UiLocalization) -> String {
    localization
        .text(language, language.localization_key())
        .to_owned()
}

pub(super) fn handle_language_dropdown_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<LanguageDropdownButton>)>,
    mut state: ResMut<LanguageDropdownState>,
) {
    if interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        state.toggle();
    }
}

pub(super) fn handle_language_options(
    interactions: Query<(&Interaction, &LanguageOption), Changed<Interaction>>,
    mut active_language: ResMut<ActiveLanguage>,
    mut state: ResMut<LanguageDropdownState>,
) {
    for (interaction, option) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if active_language.get() != option.0 {
            active_language.set(option.0);
        }
        state.close();
    }
}

pub(super) fn sync_language_dropdown(
    state: Res<LanguageDropdownState>,
    active_language: Res<ActiveLanguage>,
    localization: Res<UiLocalization>,
    mut panels: Query<&mut Node, With<LanguageDropdownPanel>>,
    mut labels: Query<&mut Text, (With<LanguageDropdownLabel>, Without<LanguageOptionLabel>)>,
    mut options: Query<(
        &LanguageOption,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut option_labels: Query<(&LanguageOptionLabel, &mut Text), Without<LanguageDropdownLabel>>,
) {
    let open_changed = state.is_changed();
    if open_changed {
        let display = if state.is_open() {
            Display::Flex
        } else {
            Display::None
        };
        for mut panel in &mut panels {
            panel.display = display;
        }
    }

    let language_changed = active_language.is_changed() || localization.is_changed();
    if language_changed {
        for mut text in &mut labels {
            text.0 = language_label(active_language.get(), &localization);
        }
    }

    if !open_changed && !language_changed && options.iter().next().is_none() {
        return;
    }

    for (option, interaction, background, border) in &mut options {
        selectable::apply_colors(
            selectable::colors(*interaction, option.0 == active_language.get()),
            background,
            border,
        );
    }

    if localization.is_changed() {
        for (option, mut text) in &mut option_labels {
            text.0 = language_label(option.0, &localization);
        }
    }
}

pub(super) fn close_language_dropdown_outside(
    mouse: Res<ButtonInput<MouseButton>>,
    inside: Query<&Interaction, With<dropdown::DropdownInside<LanguageDropdownKind>>>,
    mut state: ResMut<LanguageDropdownState>,
) {
    if dropdown::clicked_outside(state.is_open(), &mouse, &inside) {
        state.close();
    }
}
