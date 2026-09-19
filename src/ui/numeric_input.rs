use std::{fmt::Display, marker::PhantomData};

use bevy::{
    input_focus::InputFocus,
    prelude::*,
    text::{EditableText, EditableTextFilter, FontWeight},
};

use super::{
    button::COMPACT_CONTROL_HEIGHT,
    text_input::{self, editable_value},
};

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

/// The border belongs to an outer frame. The marker and editable text stay on
/// the inner Button so the existing input focus, pointer and keyboard systems
/// continue targeting exactly the same entity.
#[derive(Component)]
pub(crate) struct NumericInputFrame<I: Component>(PhantomData<I>);

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
    numeric_input_field_with_filter(
        value,
        input_marker,
        label_marker,
        sizing,
        integer_input_character,
    )
}

pub(crate) fn decimal_numeric_input_field<I: Component, L: Component>(
    value: impl Into<String>,
    input_marker: I,
    label_marker: L,
    sizing: NumericInputSizing,
) -> impl Bundle {
    numeric_input_field_with_filter(
        value,
        input_marker,
        label_marker,
        sizing,
        decimal_input_character,
    )
}

fn numeric_input_field_with_filter<I: Component, L: Component>(
    value: impl Into<String>,
    input_marker: I,
    label_marker: L,
    sizing: NumericInputSizing,
    filter: fn(char) -> bool,
) -> impl Bundle {
    let (width, flex_grow, min_width) = match sizing {
        NumericInputSizing::Fixed(width) => (px(width), 0.0, Val::Auto),
        NumericInputSizing::Flexible => (Val::Auto, 1.0, px(0)),
    };
    (
        NumericInputFrame::<I>(PhantomData),
        Node {
            width,
            flex_grow,
            min_width,
            height: px(COMPACT_CONTROL_HEIGHT),
            border: UiRect::all(px(2)),
            padding: UiRect::horizontal(px(text_input::INPUT_PADDING_X)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::FlexStart,
            overflow: Overflow::clip(),
            ..default()
        },
        text_input::frame_surface(false),
        children![(
            Button,
            input_marker,
            label_marker,
            EditableText {
                max_characters: Some(20),
                ..EditableText::new(value.into())
            },
            EditableTextFilter::new(filter),
            text_input::editor_style(20.0, FontWeight::MEDIUM),
            Node {
                width: percent(100),
                min_width: px(0),
                // The frame centers the glyph-sized editor; 100% height would
                // leave text/caret anchored at the top of a 44px field.
                height: px(26),
                overflow: Overflow::clip(),
                ..default()
            },
        )],
    )
}

fn integer_input_character(character: char) -> bool {
    character.is_ascii_digit()
}

fn decimal_input_character(character: char) -> bool {
    character.is_ascii_digit() || character == '.'
}

pub(crate) fn sync_numeric_input_view<M, I, L>(
    state: &NumericInputState<M>,
    value: impl Display,
    editors: &mut Query<&mut EditableText, With<L>>,
    inputs: &mut Query<&mut BorderColor, With<NumericInputFrame<I>>>,
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

    let next_border = BorderColor::all(text_input::input_border(state.editing()));
    for mut border in inputs {
        if *border != next_border {
            *border = next_border;
        }
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
        assert_eq!(editor.buffer(), "");
    }

    #[test]
    fn decimal_filter_accepts_digits_and_decimal_point_only() {
        assert!(decimal_input_character('0'));
        assert!(decimal_input_character('.'));
        assert!(!decimal_input_character('-'));
        assert!(!decimal_input_character('x'));
    }
}
