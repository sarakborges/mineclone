use std::{fmt::Display, marker::PhantomData};

use bevy::prelude::*;

use super::{button::COMPACT_CONTROL_HEIGHT, text_input::select_all_pressed, theme, typography};

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
pub(crate) struct NumericInputState<M>
where
    M: Send + Sync + 'static,
{
    editing: bool,
    replace_on_next_digit: bool,
    buffer: String,
    marker: PhantomData<M>,
}

impl<M> Default for NumericInputState<M>
where
    M: Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            editing: false,
            replace_on_next_digit: false,
            buffer: String::new(),
            marker: PhantomData,
        }
    }
}

impl<M> NumericInputState<M>
where
    M: Send + Sync + 'static,
{
    pub(crate) fn editing(&self) -> bool {
        self.editing
    }

    pub(crate) fn buffer(&self) -> &str {
        &self.buffer
    }

    pub(crate) fn begin(&mut self, value: impl Display) {
        self.editing = true;
        self.replace_on_next_digit = false;
        self.buffer = value.to_string();
    }

    // Preserve ResMut until an input event actually changes the editor state.
    pub(crate) fn begin_if_pressed<'a>(
        state: &mut ResMut<'_, Self>,
        mut interactions: impl Iterator<Item = &'a Interaction>,
        value: impl Display,
    ) {
        if interactions.any(|interaction| *interaction == Interaction::Pressed) {
            state.begin(value);
        }
    }

    pub(crate) fn reset(&mut self) {
        self.editing = false;
        self.replace_on_next_digit = false;
        self.buffer.clear();
    }

    pub(crate) fn display(&self, value: impl Display) -> String {
        if self.editing {
            format!("{}|", self.buffer)
        } else {
            value.to_string()
        }
    }

    pub(crate) fn handle_keyboard<F>(
        state: &mut ResMut<'_, Self>,
        keys: &ButtonInput<KeyCode>,
        max_digits: usize,
        accept_next: F,
    ) -> NumericInputEvent
    where
        F: FnOnce(&str) -> bool,
    {
        if !state.editing {
            return NumericInputEvent::None;
        }

        if select_all_pressed(keys) {
            state.replace_on_next_digit = true;
            return NumericInputEvent::None;
        }

        if keys.just_pressed(KeyCode::Enter)
            || keys.just_pressed(KeyCode::NumpadEnter)
            || keys.just_pressed(KeyCode::Escape)
        {
            state.reset();
            return NumericInputEvent::Finished;
        }

        if keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::NumpadBackspace) {
            if state.replace_on_next_digit {
                state.buffer.clear();
                state.replace_on_next_digit = false;
            } else {
                state.buffer.pop();
            }
            return NumericInputEvent::Changed;
        }

        let Some(digit) = pressed_digit(keys) else {
            return NumericInputEvent::None;
        };
        let mut next = if state.replace_on_next_digit {
            String::new()
        } else {
            state.buffer.clone()
        };
        next.push(digit);

        if next.len() > max_digits || !accept_next(&next) {
            return NumericInputEvent::None;
        }

        state.buffer = next;
        state.replace_on_next_digit = false;
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
        input_marker,
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
        children![(typography::button_label(value), label_marker)],
    )
}

pub(crate) fn sync_numeric_input_view<M, I, L>(
    state: &NumericInputState<M>,
    value: impl Display,
    labels: &mut Query<&mut Text, With<L>>,
    inputs: &mut Query<&mut BorderColor, With<I>>,
) where
    M: Send + Sync + 'static,
    I: Component,
    L: Component,
{
    let next = state.display(value);

    for mut label in labels {
        if label.0 != next {
            label.0 = next.clone();
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

fn pressed_digit(keys: &ButtonInput<KeyCode>) -> Option<char> {
    const DIGITS: [(KeyCode, char); 20] = [
        (KeyCode::Digit0, '0'),
        (KeyCode::Digit1, '1'),
        (KeyCode::Digit2, '2'),
        (KeyCode::Digit3, '3'),
        (KeyCode::Digit4, '4'),
        (KeyCode::Digit5, '5'),
        (KeyCode::Digit6, '6'),
        (KeyCode::Digit7, '7'),
        (KeyCode::Digit8, '8'),
        (KeyCode::Digit9, '9'),
        (KeyCode::Numpad0, '0'),
        (KeyCode::Numpad1, '1'),
        (KeyCode::Numpad2, '2'),
        (KeyCode::Numpad3, '3'),
        (KeyCode::Numpad4, '4'),
        (KeyCode::Numpad5, '5'),
        (KeyCode::Numpad6, '6'),
        (KeyCode::Numpad7, '7'),
        (KeyCode::Numpad8, '8'),
        (KeyCode::Numpad9, '9'),
    ];

    DIGITS
        .into_iter()
        .find_map(|(key, digit)| keys.just_pressed(key).then_some(digit))
}
