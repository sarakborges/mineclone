use bevy::{prelude::*, ui::InteractionDisabled};

use crate::{
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{
        selectable::{
            selectable_button_background, selectable_label_color, sync_selectable_button,
        },
        typography,
    },
};

const LANGUAGE_BUTTON_HEIGHT: f32 = 44.0;

#[derive(Component, Clone, Copy)]
pub(super) struct LanguageButton(pub(super) Language);

#[derive(Component, Clone, Copy)]
pub(super) struct LanguageButtonLabel(Language);

pub(super) fn languages_section(
    localization: &UiLocalization,
    active_language: Language,
) -> impl Bundle {
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
                    .text(active_language, "settings.language")
                    .to_owned(),
            ),
            typography::caption(
                localization
                    .text(active_language, "settings.language.description")
                    .to_owned(),
            ),
            language_button(
                localization
                    .text(active_language, Language::English.localization_key())
                    .to_owned(),
                Language::English,
                active_language,
            ),
        ],
    )
}

fn language_button(
    label: impl Into<String>,
    language: Language,
    active_language: Language,
) -> impl Bundle {
    let active = language == active_language;

    (
        Button,
        LanguageButton(language),
        Node {
            width: percent(100),
            height: px(LANGUAGE_BUTTON_HEIGHT),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(selectable_button_background(active, Interaction::None)),
        children![(
            typography::button_label(label),
            LanguageButtonLabel(language),
        )],
    )
}

pub(super) fn handle_language_buttons(
    interactions: Query<
        (&Interaction, &LanguageButton),
        (Changed<Interaction>, Without<InteractionDisabled>),
    >,
    mut active_language: ResMut<ActiveLanguage>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed && active_language.get() != button.0 {
            active_language.set(button.0);
        }
    }
}

pub(super) fn sync_language_buttons(
    mut commands: Commands,
    localization: Res<UiLocalization>,
    active_language: Res<ActiveLanguage>,
    mut buttons: Query<(
        Entity,
        &LanguageButton,
        &Interaction,
        Has<InteractionDisabled>,
        &mut BackgroundColor,
    )>,
    mut labels: Query<(&LanguageButtonLabel, &mut Text, &mut TextColor)>,
) {
    let active_language = active_language.get();

    for (entity, button, interaction, disabled, mut background) in &mut buttons {
        sync_selectable_button(
            &mut commands,
            entity,
            button.0 == active_language,
            disabled,
            *interaction,
            &mut background,
        );
    }

    for (label, mut text, mut color) in &mut labels {
        let active = label.0 == active_language;
        *color = TextColor(selectable_label_color(active));

        let next = localization.text(active_language, label.0.localization_key());
        if text.0 != next {
            text.0 = next.to_owned();
        }
    }
}
