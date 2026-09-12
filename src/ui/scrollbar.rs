use bevy::{
    prelude::*,
    ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb},
};

use crate::ui::theme;

#[derive(Component, Clone, Copy)]
pub(crate) struct AutoScrollbar {
    target: Entity,
}

pub(crate) fn vertical_scrollbar(target: Entity) -> impl Bundle {
    (
        AutoScrollbar { target },
        Node {
            display: Display::None,
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

pub(crate) fn sync_auto_scrollbars(
    scroll_areas: Query<&ComputedNode, Without<AutoScrollbar>>,
    mut scrollbars: Query<(&AutoScrollbar, &mut Node)>,
) {
    for (scrollbar, mut node) in &mut scrollbars {
        let Ok(computed) = scroll_areas.get(scrollbar.target) else {
            continue;
        };

        let overflowing = computed.content_size().y > computed.size().y + 0.5;
        node.display = if overflowing {
            Display::Flex
        } else {
            Display::None
        };
    }
}
