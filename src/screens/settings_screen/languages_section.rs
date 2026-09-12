use bevy::{prelude::*, ui::InteractionDisabled};

use crate::{
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{surface, theme, typography},
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
        surface::settings_section(),
        children![
            typography::heading(
                localization
                    .text(active_language, "settings.section.languages")
                    .to_owned(),
            ),
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    ..default()
                },
                children![language_button(
                    localization
                        .text(active_language, Language::English.localization_key())
                        .to_owned(),
                    Language::English,
                    active_language,
                )],
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
        BackgroundColor(language_button_background(active, Interaction::None)),
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
        let active = button.0 == active_language;

        if active && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        } else if !active && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        }

        *background = BackgroundColor(language_button_background(active, *interaction));
    }

    for (label, mut text, mut color) in &mut labels {
        let next = localization.text(active_language, label.0.localization_key());
        if text.0 != next {
            text.0 = next.to_owned();
        }
        *color = TextColor(if label.0 == active_language {
            theme::TEXT_SUBTLE
        } else {
            theme::TEXT_PRIMARY
        });
    }
}

fn language_button_background(active: bool, interaction: Interaction) -> Color {
    if active {
        return Color::srgba(0.08, 0.07, 0.12, 0.62);
    }

    match interaction {
        Interaction::Pressed => Color::srgba(0.34, 0.22, 0.62, 0.92),
        Interaction::Hovered => Color::srgba(0.29, 0.19, 0.54, 0.82),
        Interaction::None => Color::srgba(0.20, 0.14, 0.38, 0.72),
    }
}
