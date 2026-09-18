use bevy::{
    prelude::*,
    ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb},
};

use crate::ui::theme;

#[derive(Component, Clone, Copy)]
pub(crate) struct AutoScrollbar {
    target: Entity,
}

/// For capped-height scroll areas: start hidden and reveal only when the
/// laid-out content is taller than the available viewport.
pub(crate) fn vertical_scrollbar(target: Entity) -> impl Bundle {
    (AutoScrollbar { target }, scrollbar(target, true))
}

fn scrollbar(target: Entity, initially_hidden: bool) -> impl Bundle {
    (
        Interaction::default(),
        Node {
            display: if initially_hidden { Display::None } else { Display::Flex },
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
                border: UiRect::all(px(1)),
            },
        )],
    )
}

/// Uses measured visual content, not the number of elements: wrapped text
/// consumes several lines. Equality means there is nothing to scroll.
fn exceeds_viewport(content_height: f32, viewport_height: f32) -> bool {
    content_height > viewport_height + 0.5
}

pub(crate) fn sync_auto_scrollbars(
    scroll_areas: Query<&ComputedNode, Without<AutoScrollbar>>,
    mut scrollbars: Query<(&AutoScrollbar, &mut Node)>,
) {
    for (scrollbar, mut node) in &mut scrollbars {
        let Ok(computed) = scroll_areas.get(scrollbar.target) else {
            continue;
        };
        let next_display = if exceeds_viewport(computed.content_size().y, computed.size().y) {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != next_display {
            node.display = next_display;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_actual_overflow_reveals_scrollbar() {
        assert!(!exceeds_viewport(0.0, 330.0));
        assert!(!exceeds_viewport(330.0, 330.0));
        assert!(!exceeds_viewport(330.4, 330.0));
        assert!(exceeds_viewport(331.0, 330.0));
    }
}
