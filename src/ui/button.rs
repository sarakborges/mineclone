use bevy::prelude::*;

use super::typography;

pub const MENU_BUTTON_WIDTH: f32 = 470.0;
pub const MENU_BUTTON_HEIGHT: f32 = 54.0;
pub const SIDEBAR_MENU_BUTTON_HEIGHT: f32 = 54.0;
pub const COMPACT_CONTROL_HEIGHT: f32 = 44.0;

const BUTTON_NORMAL: Color = Color::srgb(0.78, 0.79, 0.80);
const BUTTON_HOVER: Color = Color::srgb(0.90, 0.91, 0.92);
const BUTTON_PRESSED: Color = Color::srgb(0.64, 0.65, 0.66);
const BUTTON_BORDER: Color = Color::srgb(0.36, 0.37, 0.38);
const BUTTON_BORDER_STRONG: Color = Color::srgb(0.96, 0.96, 0.96);
const BUTTON_SHADOW: Color = Color::srgba(0.02, 0.02, 0.02, 0.72);
const BUTTON_PRIMARY: Color = theme::PURPLE;
const BUTTON_PRIMARY_HOVER: Color = theme::PURPLE_HOVER;
const BUTTON_PRIMARY_PRESSED: Color = Color::srgb(0.38, 0.22, 0.58);
const BUTTON_DANGER: Color = theme::DANGER;
const BUTTON_DANGER_HOVER: Color = Color::srgb(0.86, 0.28, 0.28);
const BUTTON_DANGER_PRESSED: Color = Color::srgb(0.62, 0.14, 0.14);

const BUTTON_VISUAL_SETTLE_EPSILON: f32 = 0.001;

#[derive(Component, Default)]
pub struct AsteriaButtonVisual { level: f32 }

#[derive(Component, Clone, Copy, Default)]
enum ButtonVariant { #[default] Normal, Primary, Danger }

pub fn menu_button<A: Component>(label: impl Into<String>, action: A) -> impl Bundle {
    (Button, action, AsteriaButtonVisual::default(), ButtonVariant::Normal, Node { width: px(MENU_BUTTON_WIDTH), height: px(MENU_BUTTON_HEIGHT), align_items: AlignItems::Center, justify_content: JustifyContent::Center, border: UiRect::all(px(2)), ..default() }, BackgroundColor(BUTTON_NORMAL), BorderColor::all(BUTTON_BORDER), children![typography::button_label(label)])
}

pub fn sidebar_menu_button<A: Component, L: Component>(label: impl Into<String>, action: A, label_marker: L) -> impl Bundle {
    (Button, action, AsteriaButtonVisual::default(), ButtonVariant::Normal, Node { width: percent(100), height: px(SIDEBAR_MENU_BUTTON_HEIGHT), padding: UiRect::axes(px(14), px(0)), align_items: AlignItems::Center, justify_content: JustifyContent::Center, border: UiRect::all(px(2)), ..default() }, BackgroundColor(BUTTON_NORMAL), BorderColor::all(BUTTON_BORDER), children![(typography::button_label(label), label_marker)])
}

pub fn primary_menu_button<A: Component>(label: impl Into<String>, action: A) -> impl Bundle {
    (Button, action, AsteriaButtonVisual::default(), ButtonVariant::Primary, Node { width: px(MENU_BUTTON_WIDTH), height: px(MENU_BUTTON_HEIGHT), align_items: AlignItems::Center, justify_content: JustifyContent::Center, border: UiRect::all(px(2)), ..default() }, BackgroundColor(BUTTON_PRIMARY), BorderColor::all(BUTTON_BORDER_STRONG), children![typography::button_label_light(label)])
}

pub fn danger_menu_button<A: Component>(label: impl Into<String>, action: A) -> impl Bundle {
    (Button, action, AsteriaButtonVisual::default(), ButtonVariant::Danger, Node { width: px(MENU_BUTTON_WIDTH), height: px(MENU_BUTTON_HEIGHT), align_items: AlignItems::Center, justify_content: JustifyContent::Center, border: UiRect::all(px(2)), ..default() }, BackgroundColor(BUTTON_DANGER), BorderColor::all(BUTTON_BORDER_STRONG), children![typography::button_label_light(label)])
}

pub(crate) fn compact_control_button<A: Component>(label: impl Into<String>, action: A, width: f32) -> impl Bundle {
    (Button, action, ButtonVariant::Normal, Node { width: px(width), height: px(COMPACT_CONTROL_HEIGHT), align_items: AlignItems::Center, justify_content: JustifyContent::Center, border: UiRect::all(px(2)), ..default() }, BackgroundColor(BUTTON_NORMAL), BorderColor::all(BUTTON_BORDER), children![typography::button_label(label)])
}

pub fn animate_buttons(time: Res<Time<Real>>, mut buttons: Query<(&Interaction, &ButtonVariant, &mut AsteriaButtonVisual, &mut BackgroundColor, &mut BorderColor, &mut BoxShadow), With<Button>>) {
    let smoothing = 1.0 - (-14.0 * time.delta_secs()).exp();
    for (interaction, variant, mut visual, mut background, mut border, mut shadow) in &mut buttons {
        let target = match interaction { Interaction::None => 0.0, Interaction::Hovered => 1.0, Interaction::Pressed => 2.0 };
        let delta = target - visual.level;
        if delta.abs() > BUTTON_VISUAL_SETTLE_EPSILON { visual.level += delta * smoothing; } else { visual.level = target; }
        let level = visual.level.clamp(0.0, 2.0);
        let (next_background, next_border) = button_colors(level, *variant);
        if background.0 != next_background { background.0 = next_background; }
        let next_border = BorderColor::all(next_border);
        if *border != next_border { *border = next_border; }
        let next_shadow = button_shadow(level);
        if *shadow != next_shadow { *shadow = next_shadow; }
    }
}

fn button_colors(level: f32, variant: ButtonVariant) -> (Color, Color) {
    match variant {
        ButtonVariant::Normal => if level >= 1.5 { (BUTTON_PRESSED, BUTTON_BORDER_STRONG) } else if level >= 0.25 { (BUTTON_HOVER, BUTTON_BORDER_STRONG) } else { (BUTTON_NORMAL, BUTTON_BORDER) },
        ButtonVariant::Primary => if level >= 1.5 { (BUTTON_PRIMARY_PRESSED, BUTTON_BORDER_STRONG) } else if level >= 0.25 { (BUTTON_PRIMARY_HOVER, BUTTON_BORDER_STRONG) } else { (BUTTON_PRIMARY, BUTTON_BORDER_STRONG) },
        ButtonVariant::Danger => if level >= 1.5 { (BUTTON_DANGER_PRESSED, BUTTON_BORDER_STRONG) } else if level >= 0.25 { (BUTTON_DANGER_HOVER, BUTTON_BORDER_STRONG) } else { (BUTTON_DANGER, BUTTON_BORDER_STRONG) },
    }
}

fn button_shadow(level: f32) -> BoxShadow {
    let lift = level.clamp(0.0, 1.0);
    BoxShadow(vec![ShadowStyle { color: BUTTON_SHADOW, x_offset: px(0), y_offset: px(3.0 - lift), spread_radius: px(0), blur_radius: px(0) }])
}
