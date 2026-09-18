use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::{EditableText, FontWeight},
};

use crate::{
    localization::{Language, UiLocalization},
    ui::{settings as settings_layout, text_input, typography},
    world::NewWorldConfig,
};

#[derive(Component)]
pub(super) struct WorldNameFrame;

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

type WorldNameInteractions<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Interaction),
    (With<WorldNameInput>, Changed<Interaction>),
>;

pub(super) fn world_name_setting(
    config: &NewWorldConfig,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        settings_layout::setting_column(),
        children![
            typography::setting_title(localization.text(language, "newWorld.name").to_owned()),
            typography::caption(localization.text(language, "newWorld.name.description").to_owned()),
            (
                WorldNameFrame,
                Node {
                    width: percent(100),
                    height: px(44),
                    padding: UiRect::horizontal(px(text_input::INPUT_PADDING_X)),
                    border: UiRect::all(px(2)),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                text_input::frame_surface(false),
                children![(
                    Button,
                    WorldNameInput,
                    EditableText {
                        max_characters: Some(200),
                        ..EditableText::new(config.name())
                    },
                    text_input::editor_style(20.0, FontWeight::NORMAL),
                    Node {
                        width: percent(100),
                        min_width: px(0),
                        height: px(text_input::INPUT_EDITOR_HEIGHT),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                )],
            ),
            (WorldNameError, typography::caption(String::new())),
        ],
    )
}

pub(super) fn handle_world_name_focus(
    interactions: WorldNameInteractions,
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
    input: Query<Entity, With<WorldNameInput>>,
    mut frames: Query<&mut BorderColor, With<WorldNameFrame>>,
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
    let focused = input
        .single()
        .is_ok_and(|entity| focus.get() == Some(entity));
    let next = BorderColor::all(text_input::input_border(focused));
    for mut border in &mut frames {
        if *border != next {
            *border = next;
        }
    }
}
