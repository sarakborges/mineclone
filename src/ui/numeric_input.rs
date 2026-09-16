use std::{fmt::Display, marker::PhantomData};

use bevy::{
    input_focus::InputFocus,
    prelude::*,
    text::{EditableText, EditableTextFilter, FontWeight, TextCursorStyle},
    ui_widgets::TextInput,
};

use super::{button::COMPACT_CONTROL_HEIGHT, text_input::editable_value, theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumericInputEvent {
    None,
    Changed,
    Finished,
}

#[derive(Clone, Copy)]
pub(crate) enum NumericInputSizing {
    Fixed(f32),
    Flexible,
}

#[derive(Resource)]
pub(crate) struct NumericInputState<M: Send + Sync + 'static> {
    editing: bool,
    buffer: String,
    marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> Default for NumericInputState<M> {
    fn default() -> Self {
        Self {
            editing: false,
            buffer: String::new(),
            marker: PhantomData,
        }
    }
}

impl<M: Send + Sync + 'static> NumericInputState<M> {
    pub(crate) fn editing(&self) -> bool {
        self.editing
    }

    pub(crate) fn buffer(&self) -> &str {
        &self.buffer
    }

    pub(crate) fn begin(&mut self, value: impl Display) {
        self.editing = true;
        self.buffer = value.to_string();
    }

    pub(crate) fn begin_if_pressed<'a>(
        state: &mut ResMut<'_, Self>,
        interactions: impl Iterator<Item = &'a Interaction>,
        value: impl Display,
    ) {
        if !state.editing && interactions.into_iter().any(|i| *i == Interaction::Pressed) {
            state.begin(value);
        }
    }

    pub(crate) fn reset(&mut self) {
        self.editing = false;
        self.buffer.clear();
    }

    /// The built-in editor handles OS shortcuts, selection, clipboard and caret.
    /// This adapter only validates its committed numeric contents and commits the value.
    pub(crate) fn handle_keyboard<F>(
        state: &mut ResMut<'_, Self>,
        keys: &ButtonInput<KeyCode>,
        focus: &mut InputFocus,
        entity: Entity,
        editor: &mut EditableText,
        max_digits: usize,
        accept_next: F,
    ) -> NumericInputEvent
    where
        F: FnOnce(&str) -> bool,
    {
        if !state.editing {
            if focus.get() == Some(entity) {
                focus.clear();
            }
            return NumericInputEvent::None;
        }
        if focus.get() != Some(entity) {
            state.reset();
            return NumericInputEvent::None;
        }
        if editor.is_composing() {
            return NumericInputEvent::None;
        }
        if keys.just_pressed(KeyCode::Enter)
            || keys.just_pressed(KeyCode::NumpadEnter)
            || keys.just_pressed(KeyCode::Escape)
        {
            state.reset();
            focus.clear();
            return NumericInputEvent::Finished;
        }

        let next = editable_value(editor);
        if next == state.buffer {
            return NumericInputEvent::None;
        }
        if next.chars().count() > max_digits || !accept_next(&next) {
            editor.editor_mut().set_text(&state.buffer);
            return NumericInputEvent::None;
        }
        state.buffer = next;
        NumericInputEvent::Changed
    }
}

pub(crate) fn numeric_input_field<I: Component, L: Component>(
    value: impl Into<String>,
    input_marker: I,
    label_marker: L,
    sizing: NumericInputSizing,
) -> impl Bundle {
    let (width, flex_grow, min_width) = match sizing {
        NumericInputSizing::Fixed(width) => (px(width), 0.0, Val::Auto),
        NumericInputSizing::Flexible => (Val::Auto, 1.0, px(0)),
    };
    (
        Button,
        TextInput,
        input_marker,
        label_marker,
        EditableText {
            max_characters: Some(20),
            ..EditableText::new(value.into())
        },
        EditableTextFilter::new(|character| character.is_ascii_digit()),
        TextLayout::no_wrap(),
        TextFont {
            font: FontSource::SystemUi,
            font_size: FontSize::Px(20.0),
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(theme::TEXT_PRIMARY),
        TextCursorStyle { color: theme::TEXT_PRIMARY, ..default() },
        Node {
            width,
            flex_grow,
            min_width,
            height: px(COMPACT_CONTROL_HEIGHT),
            border: UiRect::all(px(1)),
            padding: UiRect::axes(px(14), px(0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::FlexStart,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.045, 0.035, 0.09, 0.88)),
        BorderColor::all(numeric_input_border(false)),
    )
}

pub(crate) fn sync_numeric_input_view<M, I, L>(
    state: &NumericInputState<M>,
    value: impl Display,
    editors: &mut Query<&mut EditableText, With<L>>,
    inputs: &mut Query<&mut BorderColor, With<I>>,
) where
    M: Send + Sync + 'static,
    I: Component,
    L: Component,
{
    if !state.editing() {
        let next = value.to_string();
        for mut editor in editors {
            if editable_value(&editor) != next {
                editor.editor_mut().set_text(&next);
            }
        }
    }

    let next_border = BorderColor::all(numeric_input_border(state.editing()));
    for mut border in inputs {
        if *border != next_border {
            *border = next_border;
        }
    }
}

fn numeric_input_border(editing: bool) -> Color {
    if editing {
        theme::TEXT_PRIMARY.with_alpha(0.92)
    } else {
        Color::srgba(0.43, 0.36, 0.68, 0.72)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Seed;

    #[test]
    fn numeric_model_preserves_focus_and_buffer_until_blurred() {
        let mut editor = NumericInputState::<Seed>::default();
        editor.begin(123_u64);
        assert!(editor.editing());
        assert_eq!(editor.buffer(), "123");
        editor.reset();
        assert!(!editor.editing());
        assert_eq!(editor.buffer(), "");
    }
}
