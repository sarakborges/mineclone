use bevy::{input_focus::{FocusCause, InputFocus}, prelude::*, text::{EditableText, TextCursorStyle}};

use crate::{
    localization::{Language, UiLocalization},
    ui::{text_input, theme, typography},
    world::NewWorldConfig,
};

#[derive(Component)]
pub(super) struct WorldNameInput;

#[derive(Component)]
pub(super) struct WorldNameError;

#[derive(Resource, Default)]
pub(super) struct WorldNameFeedback {
    message: String,
}

impl WorldNameFeedback {
    pub(super) fn set(&mut self, message: String) {
        if self.message != message {
            self.message = message;
        }
    }
}

pub(super) fn world_name_setting(
    config: &NewWorldConfig,
    localization: &UiLocalization,
    language: Language,
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
            typography::setting_title(localization.text(language, "newWorld.name").to_owned()),
            typography::caption(localization.text(language, "newWorld.name.description").to_owned()),
            (
                Button,
                WorldNameInput,
                EditableText {
                    max_characters: Some(200),
                    ..EditableText::new(config.name())
                },
                TextFont {
                    font: FontSource::SystemUi,
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(theme::TEXT_PRIMARY),
                TextCursorStyle {
                    color: theme::TEXT_PRIMARY,
                    ..default()
                },
                TextLayout::no_wrap(),
                Node {
                    width: percent(100),
                    height: px(44),
                    padding: UiRect::horizontal(px(text_input::INPUT_PADDING_X)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(text_input::INPUT_RADIUS)),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(text_input::INPUT_FILL),
                BorderColor::all(text_input::input_border(false)),
            ),
            (
                WorldNameError,
                typography::caption(String::new()),
                TextColor(theme::TEXT_PRIMARY),
            ),
        ],
    )
}

pub(super) fn handle_world_name_focus(
    interactions: Query<(Entity, &Interaction), (With<WorldNameInput>, Changed<Interaction>)>,
    mut focus: ResMut<InputFocus>,
) {
    for (entity, interaction) in &interactions {
        if *interaction == Interaction::Pressed {
            focus.set(entity, FocusCause::Pressed);
        }
    }
}

pub(super) fn sync_world_name_view(
    feedback: Res<WorldNameFeedback>,
    focus: Res<InputFocus>,
    mut inputs: Query<(Entity, &mut BorderColor), With<WorldNameInput>>,
    mut errors: Query<&mut Text, With<WorldNameError>>,
) {
    if !feedback.is_changed() && !focus.is_changed() {
        return;
    }
    for mut text in &mut errors {
        if text.0 != feedback.message {
            text.0.clone_from(&feedback.message);
        }
    }
    for (entity, mut border) in &mut inputs {
        let next = BorderColor::all(text_input::input_border(focus.get() == Some(entity)));
        if *border != next {
            *border = next;
        }
    }
}
