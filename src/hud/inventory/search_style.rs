use bevy::prelude::*;

use crate::ui::{text_input, theme};

use super::state::{CreativeInventoryView, CreativeSearchBar, CreativeSearchText, SEARCH_HEIGHT};

type SearchInputs<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut BackgroundColor, &'static mut BorderColor),
    (With<CreativeSearchBar>, Without<CreativeSearchText>),
>;
type SearchPlaceholders<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut TextColor),
    (With<CreativeSearchText>, Without<CreativeSearchBar>),
>;

/// Keep the placeholder outside the editable text's flex layout and use shared styling.
pub(super) fn style_inventory_search_field(
    view: Res<CreativeInventoryView>,
    mut inputs: SearchInputs,
    mut placeholders: SearchPlaceholders,
) {
    let padding = UiRect::horizontal(px(text_input::INPUT_PADDING_X));
    let radius = BorderRadius::all(px(text_input::INPUT_RADIUS));
    let clipped = Overflow::clip();
    let fill = BackgroundColor(text_input::INPUT_FILL);
    let border = BorderColor::all(text_input::input_border(view.search_focused()));

    for (mut node, mut background, mut input_border) in &mut inputs {
        if node.padding != padding {
            node.padding = padding;
        }
        if node.border_radius != radius {
            node.border_radius = radius;
        }
        if node.overflow != clipped {
            node.overflow = clipped;
        }
        if *background != fill {
            *background = fill;
        }
        if *input_border != border {
            *input_border = border;
        }
    }

    for (mut node, mut color) in &mut placeholders {
        let left = px(text_input::INPUT_PADDING_X);
        let top = px((SEARCH_HEIGHT - 17.0) * 0.5);
        if node.position_type != PositionType::Absolute {
            node.position_type = PositionType::Absolute;
        }
        if node.left != left {
            node.left = left;
        }
        if node.top != top {
            node.top = top;
        }
        if color.0 != theme::TEXT_MUTED {
            color.0 = theme::TEXT_MUTED;
        }
    }
}
