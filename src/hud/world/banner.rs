use bevy::prelude::*;

pub(super) fn world_banner() -> impl Bundle {
    (
        Node {
            width: px(520),
            max_width: percent(80),
            padding: UiRect::axes(px(28), px(5)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(1),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        BackgroundGradient::from(LinearGradient::to_right(vec![
            ColorStop::percent(Color::srgba(0.16, 0.17, 0.20, 0.0), 0.0),
            ColorStop::percent(Color::srgba(0.16, 0.17, 0.20, 0.10), 16.0),
            ColorStop::percent(Color::srgba(0.15, 0.16, 0.19, 0.22), 36.0),
            ColorStop::percent(Color::srgba(0.14, 0.15, 0.18, 0.30), 50.0),
            ColorStop::percent(Color::srgba(0.15, 0.16, 0.19, 0.22), 64.0),
            ColorStop::percent(Color::srgba(0.16, 0.17, 0.20, 0.10), 84.0),
            ColorStop::percent(Color::srgba(0.16, 0.17, 0.20, 0.0), 100.0),
        ])),
    )
}
