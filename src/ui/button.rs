use bevy::prelude::*;

use super::{theme, typography};

pub const MENU_BUTTON_WIDTH: f32 = 470.0;
pub const MENU_BUTTON_HEIGHT: f32 = 54.0;
pub const COMPACT_CONTROL_HEIGHT: f32 = 44.0;

const BUTTON_NORMAL: Color = theme::SURFACE_ELEVATED;
const BUTTON_PRESSED: Color = theme::PURPLE;
const BUTTON_BORDER: Color = theme::BORDER;
const BUTTON_BORDER_STRONG: Color = theme::BORDER_STRONG;
const BUTTON_SHADOW: Color = Color::srgba(0.02, 0.02, 0.02, 0.72);
const BUTTON_PRIMARY: Color = theme::PURPLE;
const BUTTON_PRIMARY_HOVER: Color = theme::PURPLE_HOVER;
const BUTTON_PRIMARY_PRESSED: Color = Color::srgb(0.38, 0.22, 0.58);
const BUTTON_DANGER: Color = theme::DANGER;
const BUTTON_DANGER_HOVER: Color = theme::DANGER_HOVER;
const BUTTON_DANGER_PRESSED: Color = theme::DANGER_PRESSED;

const BUTTON_VISUAL_SETTLE_EPSILON: f32 = 0.001;

#[derive(Component, Default)]
pub struct AsteriaButtonVisual { level: f32 }

#[derive(Component, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonVariant { #[default] Normal, Primary, Danger }

impl ButtonVariant {
    pub(crate) fn from_active(active: bool) -> Self {
        if active { Self::Primary } else { Self::Normal }
    }
}

pub(crate) fn initial_button_background(active: bool) -> Color {
    if active { BUTTON_PRIMARY } else { BUTTON_NORMAL }
}

pub(crate) fn initial_button_border(active: bool) -> Color {
    if active { BUTTON_BORDER_STRONG } else { BUTTON_BORDER }
}

type ButtonAnimationQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        &'static ButtonVariant,
        &'static mut AsteriaButtonVisual,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
        Option<&'static mut BoxShadow>,
    ),
    With<Button>,
>;

pub fn button<A: Component>(label: impl Into<String>, action: A) -> impl Bundle {
    (Button, action, AsteriaButtonVisual::default(), ButtonVariant::Normal, Node { width: px(MENU_BUTTON_WIDTH), height: px(MENU_BUTTON_HEIGHT), align_items: AlignItems::Center, justify_content: JustifyContent::Center, border: UiRect::all(px(2)), ..default() }, BackgroundColor(BUTTON_NORMAL), BorderColor::all(BUTTON_BORDER), children![typography::button_label_light(label)])
}

pub fn button_with_marker<A: Component, L: Component>(label: impl Into<String>, action: A, label_marker: L) -> impl Bundle {
    (Button, action, AsteriaButtonVisual::default(), ButtonVariant::Normal, Node { width: percent(100), height: px(MENU_BUTTON_HEIGHT), padding: UiRect::axes(px(14), px(0)), align_items: AlignItems::Center, justify_content: JustifyContent::Center, border: UiRect::all(px(2)), ..default() }, BackgroundColor(BUTTON_NORMAL), BorderColor::all(BUTTON_BORDER), children![(typography::button_label(label), label_marker)])
}

pub fn primary_button<A: Component>(label: impl Into<String>, action: A) -> impl Bundle {
    (Button, action, AsteriaButtonVisual::default(), ButtonVariant::Primary, Node { width: px(MENU_BUTTON_WIDTH), height: px(MENU_BUTTON_HEIGHT), align_items: AlignItems::Center, justify_content: JustifyContent::Center, border: UiRect::all(px(2)), ..default() }, BackgroundColor(BUTTON_PRIMARY), BorderColor::all(BUTTON_BORDER_STRONG), children![typography::button_label_light(label)])
}

pub(crate) fn danger_button<A: Component>(label: impl Into<String>, action: A) -> impl Bundle {
    (Button, action, AsteriaButtonVisual::default(), ButtonVariant::Danger, Node { width: px(MENU_BUTTON_WIDTH), height: px(MENU_BUTTON_HEIGHT), align_items: AlignItems::Center, justify_content: JustifyContent::Center, border: UiRect::all(px(2)), ..default() }, BackgroundColor(BUTTON_DANGER), BorderColor::all(BUTTON_BORDER_STRONG), children![typography::button_label_light(label)])
}

pub(crate) fn standard_button_with_marker<A: Component, M: Component>(
    label: impl Into<String>,
    action: A,
    width: f32,
    variant: ButtonVariant,
    marker: M,
) -> impl Bundle {
    let (background, border) = match variant {
        ButtonVariant::Normal => (BUTTON_NORMAL, BUTTON_BORDER),
        ButtonVariant::Primary => (BUTTON_PRIMARY, BUTTON_BORDER_STRONG),
        ButtonVariant::Danger => (BUTTON_DANGER, BUTTON_BORDER_STRONG),
    };
    (
        Button,
        action,
        AsteriaButtonVisual::default(),
        variant,
        Node {
            width: px(width),
            flex_grow: if width <= 0.0 { 1.0 } else { 0.0 },
            height: px(COMPACT_CONTROL_HEIGHT),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(2)),
            ..default()
        },
        BackgroundColor(background),
        BorderColor::all(border),
        children![(typography::button_label_light(label), marker)],
    )
}

pub(crate) fn standard_button<A: Component>(
    label: impl Into<String>,
    action: A,
    width: f32,
    variant: ButtonVariant,
) -> impl Bundle {
    let (background, border) = match variant {
        ButtonVariant::Normal => (BUTTON_NORMAL, BUTTON_BORDER),
        ButtonVariant::Primary => (BUTTON_PRIMARY, BUTTON_BORDER_STRONG),
        ButtonVariant::Danger => (BUTTON_DANGER, BUTTON_BORDER_STRONG),
    };
    (
        Button,
        action,
        AsteriaButtonVisual::default(),
        variant,
        Node {
            width: px(width),
            height: px(COMPACT_CONTROL_HEIGHT),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(2)),
            ..default()
        },
        BackgroundColor(background),
        BorderColor::all(border),
        children![typography::button_label_light(label)],
    )
}

pub(crate) fn compact_control_button<A: Component>(label: impl Into<String>, action: A, width: f32) -> impl Bundle {
    (Button, action, AsteriaButtonVisual::default(), ButtonVariant::Normal, Node { width: px(width), height: px(COMPACT_CONTROL_HEIGHT), align_items: AlignItems::Center, justify_content: JustifyContent::Center, border: UiRect::all(px(2)), ..default() }, BackgroundColor(BUTTON_NORMAL), BorderColor::all(BUTTON_BORDER), children![typography::button_label(label)])
}

pub fn animate_buttons(
    time: Res<Time<Real>>,
    mut buttons: ButtonAnimationQuery,
) {
    let smoothing = 1.0 - (-14.0 * time.delta_secs()).exp();
    for (interaction, variant, mut visual, mut background, mut border, shadow) in &mut buttons {
        let target = match interaction { Interaction::None => 0.0, Interaction::Hovered => 1.0, Interaction::Pressed => 2.0 };
        let delta = target - visual.level;
        if delta.abs() > BUTTON_VISUAL_SETTLE_EPSILON { visual.level += delta * smoothing; } else { visual.level = target; }
        let level = visual.level.clamp(0.0, 2.0);
        let (next_background, next_border) = button_colors(level, *variant);
        if background.0 != next_background { background.0 = next_background; }
        let next_border = BorderColor::all(next_border);
        if *border != next_border { *border = next_border; }
        if let Some(mut shadow) = shadow {
            let next_shadow = button_shadow(level);
            if *shadow != next_shadow { *shadow = next_shadow; }
        }
    }
}

fn button_colors(level: f32, variant: ButtonVariant) -> (Color, Color) {
    match variant {
        ButtonVariant::Normal => if level >= 1.5 { (BUTTON_PRESSED, BUTTON_BORDER_STRONG) } else if level >= 0.25 { (BUTTON_PRIMARY, BUTTON_BORDER_STRONG) } else { (BUTTON_NORMAL, BUTTON_BORDER) },
        ButtonVariant::Primary => if level >= 1.5 { (BUTTON_PRIMARY_PRESSED, BUTTON_BORDER_STRONG) } else if level >= 0.25 { (BUTTON_PRIMARY_HOVER, BUTTON_BORDER_STRONG) } else { (BUTTON_PRIMARY, BUTTON_BORDER_STRONG) },
        ButtonVariant::Danger => if level >= 1.5 { (BUTTON_DANGER_PRESSED, BUTTON_BORDER_STRONG) } else if level >= 0.25 { (BUTTON_DANGER_HOVER, BUTTON_BORDER_STRONG) } else { (BUTTON_DANGER, BUTTON_BORDER_STRONG) },
    }
}

fn button_shadow(level: f32) -> BoxShadow {
    let lift = level.clamp(0.0, 1.0);
    BoxShadow(vec![ShadowStyle { color: BUTTON_SHADOW, x_offset: px(0), y_offset: px(3.0 - lift), spread_radius: px(0), blur_radius: px(0) }])
}
