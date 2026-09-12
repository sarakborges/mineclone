use bevy::prelude::*;

use super::typography;

pub const MENU_BUTTON_WIDTH: f32 = 470.0;
pub const MENU_BUTTON_HEIGHT: f32 = 54.0;

#[derive(Component, Default)]
pub struct AsteriaButtonVisual {
    level: f32,
}

pub fn menu_button<A: Component>(label: impl Into<String>, action: A) -> impl Bundle {
    (
        Button,
        action,
        AsteriaButtonVisual::default(),
        Node {
            width: px(MENU_BUTTON_WIDTH),
            height: px(MENU_BUTTON_HEIGHT),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        button_gradient(0.0),
        button_shadow(0.0),
        children![typography::button_label(label)],
    )
}

pub fn animate_buttons(
    time: Res<Time<Real>>,
    mut buttons: Query<
        (
            &Interaction,
            &mut AsteriaButtonVisual,
            &mut BackgroundGradient,
            &mut BoxShadow,
        ),
        With<Button>,
    >,
) {
    let smoothing = 1.0 - (-10.0 * time.delta_secs()).exp();

    for (interaction, mut visual, mut gradient, mut shadow) in &mut buttons {
        let target = match interaction {
            Interaction::None => 0.0,
            Interaction::Hovered => 1.0,
            Interaction::Pressed => 1.25,
        };

        visual.level += (target - visual.level) * smoothing;
        *gradient = button_gradient(visual.level);
        *shadow = button_shadow(visual.level);
    }
}

fn button_gradient(level: f32) -> BackgroundGradient {
    let lift = level.clamp(0.0, 1.25);

    BackgroundGradient::from(LinearGradient::to_right(vec![
        ColorStop::percent(Color::srgba(0.35, 0.12, 0.70, 0.0), 0.0),
        ColorStop::percent(
            Color::srgba(0.38 + 0.04 * lift, 0.11, 0.72, 0.08 + 0.04 * lift),
            13.0,
        ),
        ColorStop::percent(
            Color::srgba(0.40 + 0.05 * lift, 0.13, 0.75, 0.34 + 0.10 * lift),
            29.0,
        ),
        ColorStop::percent(
            Color::srgba(
                0.35 + 0.07 * lift,
                0.14,
                0.72 + 0.04 * lift,
                0.72 + 0.12 * lift,
            ),
            49.0,
        ),
        ColorStop::percent(
            Color::srgba(
                0.26 + 0.06 * lift,
                0.20,
                0.62 + 0.08 * lift,
                0.68 + 0.14 * lift,
            ),
            58.0,
        ),
        ColorStop::percent(
            Color::srgba(
                0.18,
                0.28 + 0.05 * lift,
                0.58 + 0.08 * lift,
                0.30 + 0.10 * lift,
            ),
            76.0,
        ),
        ColorStop::percent(Color::srgba(0.14, 0.35, 0.62, 0.06 + 0.04 * lift), 90.0),
        ColorStop::percent(Color::srgba(0.14, 0.35, 0.62, 0.0), 100.0),
    ]))
}

fn button_shadow(level: f32) -> BoxShadow {
    let lift = level.clamp(0.0, 1.25);
    BoxShadow(vec![ShadowStyle {
        color: Color::srgba(
            0.40 - 0.08 * lift,
            0.20 + 0.06 * lift,
            1.0,
            0.10 + 0.16 * lift,
        ),
        x_offset: px(0),
        y_offset: px(0),
        spread_radius: px(-4),
        blur_radius: px(18.0 + 6.0 * lift),
    }])
}
