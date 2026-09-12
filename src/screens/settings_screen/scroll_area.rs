use bevy::{prelude::*, ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb}};

use crate::ui::theme;

pub(super) fn vertical_scrollbar(target: Entity) -> impl Bundle {
    (
        Node {
            min_width: px(8),
            margin: UiRect::left(px(6)),
            grid_column: GridPlacement::start(2),
            grid_row: GridPlacement::start(1),
            ..default()
        },
        Scrollbar {
            target,
            orientation: ControlOrientation::Vertical,
            min_thumb_length: 28.0,
        },
        children![(
            BackgroundColor(theme::TEXT_SUBTLE.with_alpha(0.38)),
            BorderColor::all(theme::TEXT_SUBTLE.with_alpha(0.24)),
            ScrollbarThumb {
                border_radius: BorderRadius::all(px(4)),
                border: UiRect::all(px(1)),
            },
        )],
    )
}
